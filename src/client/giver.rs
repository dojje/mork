use std::{
    error::{self, Error},
    fs::File,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use shared::messages::{
    have_file::HaveFile, taker_ip::TakerIp, you_have_file::YouHaveFile, Message,
};
use tokio::{net::UdpSocket, time};

use crate::{
    ensure_global_ip,
    giver::{send_burst::send_file_burst, send_index::send_file_index},
    recv, SendMethod,
};

mod send_burst;
mod send_index;

fn get_file_len(file_name: &String) -> Result<u64, Box<dyn error::Error>> {
    let file = File::open(file_name)?;

    Ok(file.metadata().unwrap().len())
}

async fn send_unil_recv(
    sock: &UdpSocket,
    msg: &[u8],
    addr: &SocketAddr,
) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let mut interval = time::interval(Duration::from_millis(1500));
    let msg_buf = loop {
        tokio::select! {
            _ = interval.tick() => {
                sock.send_to(&msg, addr).await?;
            }

            result = recv(&sock, &addr) => {
                let msg_buf = result?;
                break msg_buf;
                // let you_have_file = YouHaveFile::from_raw(msg_buf.as_slice())?;
                // break you_have_file;
            }
        }
    };

    Ok(msg_buf)
}

pub async fn sender(
    file_name: String,
    sock: Arc<UdpSocket>,
    server_addr: SocketAddr,
    send_method: SendMethod,
) -> Result<(), Box<dyn Error>> {
    let file_len = match get_file_len(&file_name) {
        Ok(f) => f,
        Err(_) => panic!("file {} doesn't exist", file_name),
    };
    let have_file = HaveFile::new(file_name.clone(), file_len);

    let msg_buf = send_unil_recv(&sock, &have_file.to_raw(), &server_addr).await?;
    let you_have_file = YouHaveFile::from_raw(&msg_buf)?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    // Wait for taker ip
    loop {
        let msg_buf = send_unil_recv(&sock, &[255u8], &server_addr).await?;
        let taker_ip = TakerIp::from_raw(msg_buf.as_slice())?;

        let correct_ip = ensure_global_ip(taker_ip.ip, &server_addr);
        let file_name = file_name.clone();
        let sock_send = sock.clone();
        let send_method = send_method.clone();
        tokio::spawn(async move {
            match &send_method {
                SendMethod::Burst => {
                    send_file_burst(sock_send, file_name, correct_ip)
                        .await
                        .expect("could not send file");
                }
                SendMethod::Confirm => todo!(),
                SendMethod::Index => {
                    send_file_index(sock_send, file_name, correct_ip)
                        .await
                        .expect("could not send file");
                }
            }
        });
    }
}

fn get_buf(msg_num: &u64, file_buf: &[u8]) -> Vec<u8> {
    let msg_num_u8 = msg_num.to_be_bytes();

    let full = [&msg_num_u8, file_buf].concat();

    full
}

// ** THE PROGRESS TRACKER **
//
// It works by having an array of bits of all messages
// It's the lengh of all messages that should be recieved
// When a message is recieved, the bit for that message will flip
