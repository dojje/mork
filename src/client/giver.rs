use std::{
    error::{self, Error},
    fs::File,
    net::SocketAddr,
    sync::Arc,
    thread,
    time::Duration,
};

use log::info;
use shared::messages::{
    have_file::HaveFile, taker_ip::TakerIp, you_have_file::YouHaveFile, Message,
};
use tokio::{net::UdpSocket, time};

use crate::{ensure_global_ip, punch_hole, read_position, recv, SendMethod};

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

async fn send_file_burst(
    sock: Arc<UdpSocket>,
    file_name: String,
    reciever: SocketAddr,
) -> Result<(), Box<dyn error::Error>> {
    info!("reciever ip: {}", reciever);

    punch_hole(&sock, reciever).await?;

    thread::sleep(Duration::from_millis(1000));

    // Udp messages should be 508 bytes
    // 8 of those bytes are used for checking order of recieving bytes
    // The rest 500 bytes are used to send the file
    // The file gets send 500 bytes
    let input_file = File::open(file_name)?;
    let file_len = input_file.metadata()?.len();
    let mut offset = 0;
    info!(
        "will send {} bytes in {} packets",
        file_len,
        file_len / 500 + 1
    );
    loop {
        let mut file_buf = [0u8; 500];
        let amt = read_position(&input_file, &mut file_buf, offset)?;

        sock.send_to(&file_buf[0..amt], reciever).await?;

        offset += 500;
        if offset >= file_len {
            break;
        }
    }
    info!("done sending file");

    Ok(())
}

async fn send_file_index(
    sock: Arc<UdpSocket>,
    file_name: String,
    reciever: SocketAddr,
) -> Result<(), Box<dyn error::Error>> {
    info!("reciever ip: {}", reciever);

    punch_hole(&sock, reciever).await?;

    thread::sleep(Duration::from_millis(1000));

    // Udp messages should be 508 bytes
    // 8 of those bytes are used for checking order of recieving bytes
    // The rest 500 bytes are used to send the file
    // The file gets send 500 bytes
    let input_file = File::open(file_name)?;
    let file_len = input_file.metadata()?.len();
    let mut offset = 0;
    let mut msg_num: u64 = 0;
    info!(
        "will send {} bytes in {} packets",
        file_len,
        file_len / 500 + 1
    );
    loop {
        let mut file_buf = [0u8; 500];
        let amt = read_position(&input_file, &mut file_buf, offset)?;

        let buf = get_buf(&msg_num, &file_buf[0..amt]);
        sock.send_to(buf.as_slice(), reciever).await?;

        offset += 500;
        if offset >= file_len {
            break;
        }

        msg_num += 1;
    }
    info!("done sending file");

    Ok(())
}

// ** THE PROGRESS TRACKER **
//
// It works by having an array of bits of all messages
// It's the lengh of all messages that should be recieved
// When a message is recieved, the bit for that message will flip
