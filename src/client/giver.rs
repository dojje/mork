use std::{net::SocketAddr, error::{Error, self}, time::Duration, thread, fs::{File}, sync::Arc, };
#[cfg(target_os = "windows")]
use std::os::windows::prelude::FileExt;
#[cfg(target_os = "linux")]
use std::os::unix::fs::FileExt;

use log::info;
use shared::{messages::{have_file::HaveFile, you_have_file::{YouHaveFile}, taker_ip::{TakerIp}, Message}, send_msg};
use tokio::{net::UdpSocket, time};

use crate::{recv, punch_hole, ensure_global_ip};


pub async fn sender(file_name: String, sock: Arc<UdpSocket>, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let have_file = HaveFile::new(file_name.clone());

    // This will be used for all udp pings
    let mut interval = time::interval(Duration::from_millis(1500));
    let you_have_file = loop {
        tokio::select! {
            _ = interval.tick() => {
                info!("contacting server");
                send_msg(&sock, &have_file, server_addr).await?;
            }
            
            result = recv(&sock, server_addr) => {
                let msg_buf = result?;
                let you_have_file = YouHaveFile::from_raw(msg_buf.as_slice())?;
                break you_have_file;
            }
        }
    };

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    // Wait for taker ip
    loop {
        let sock_recv = sock.clone();
        tokio::select! {
            _ = interval.tick() => {
                // keep hole punched to server
                sock.send_to(&[255u8], server_addr).await?;
            }
            
            result = recv(&sock_recv, server_addr) => {
                let msg_buf = result?;
                let taker_ip = TakerIp::from_raw(msg_buf.as_slice())?;

                let correct_ip = ensure_global_ip(taker_ip.ip, &server_addr);
                let file_name = file_name.clone();
                let sock_send = sock.clone();
                tokio::spawn(async move {
                    send_file_to(sock_send, file_name, correct_ip).await.expect("could not send file");
                });
                // send_file_to(sock.clone(), )
            }
        }
    };
}

fn get_buf(msg_num: &u64, file_buf: &[u8]) -> Vec<u8> {
    let msg_num_u8 = msg_num.to_be_bytes();

    let full = [&msg_num_u8, file_buf].concat();

    full
}

async fn send_file_to(sock: Arc<UdpSocket>, file_name: String, reciever: SocketAddr) -> Result<(), Box<dyn error::Error>> {
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
    info!("will send {} bytes in {} packets", file_len, file_len / 500 + 1);
    loop {
        let mut file_buf = [0u8;500];
        #[cfg(target_os = "linux")]
        let amt = input_file.read_at(&mut file_buf, offset)?;

        #[cfg(target_os = "windows")]
        let amt = input_file.seek_read(&mut file_buf, offset)?;

        let buf = get_buf(&msg_num, &file_buf[0..amt]);
        sock.send_to(&buf[..], reciever).await?;

        offset += 500;
        if offset >= file_len {
            break;
        }

        msg_num += 1;
    }
    info!("done sending file");

    Ok(())
}
