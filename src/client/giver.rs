use std::{
    error::{self, Error},
    fs::File,
    net::SocketAddr,
    sync::Arc,
};

use dovepipe::send_file;
use shared::messages::{
    have_file::HaveFile, taker_ip::TakerIp, you_have_file::YouHaveFile, Message,
};
use tokio::net::UdpSocket;

use crate::{
    ensure_global_ip,
    send_unil_recv, SendMethod,
};

// mod send_burst;
// mod send_index;

fn get_file_len(file_name: &String) -> Result<u64, Box<dyn error::Error>> {
    let file = File::open(file_name)?;

    Ok(file.metadata().unwrap().len())
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

    let mut buf = [0u8; 508];
    let amt = send_unil_recv(&sock, &have_file.to_raw(), &server_addr, &mut buf, 500).await?;
    let buf = &buf[0..amt];
    let you_have_file = YouHaveFile::from_raw(&buf)?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    // Wait for taker ip
    loop {
        let mut buf = [0; 508];
        let amt = send_unil_recv(&sock, &[255u8], &server_addr, &mut buf, 1000).await?;
        let buf = &buf[0..amt];

        let taker_ip = TakerIp::from_raw(buf)?;

        let correct_ip = ensure_global_ip(taker_ip.ip, &server_addr);
        let file_name = file_name.clone();
        let sock_send = sock.clone();
        let send_method = send_method.clone();
        tokio::spawn(async move {
            match &send_method {
                SendMethod::Burst => {
                    // send_file_burst(sock_send, file_name, correct_ip)
                    //    .await
                    //    .expect("could not send file");
                }
                SendMethod::Confirm => todo!(),
                SendMethod::Index => {
                    send_file(sock_send, file_name, correct_ip)
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
