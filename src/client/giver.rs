use std::{net::SocketAddr, error::{Error, self}, time::Duration, thread, fs::{File}, sync::Arc, };
#[cfg(target_os = "windows")]
use std::os::windows::prelude::FileExt;
#[cfg(target_os = "linux")]
use std::os::unix::fs::FileExt;

use log::info;
use shared::{messages::{have_file::HaveFile, you_have_file::{YouHaveFile}, taker_ip::{TakerIp}, Message}, send_msg};
use tokio::{net::UdpSocket, time};

use crate::{recv, punch_hole};


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

                let file_name = file_name.clone();
                let sock_send = sock.clone();
                tokio::spawn(async move {
                    send_file_to(sock_send, file_name, taker_ip.ip).await.expect("could not send file");
                });
                // send_file_to(sock.clone(), )
            }
        }
    };
}

async fn send_file_to(sock: Arc<UdpSocket>, file_name: String, reciever: SocketAddr) -> Result<(), Box<dyn error::Error>> {
    println!("reciever ip: {}", reciever);

    punch_hole(&sock, reciever).await?;
    info!("punched hole to {}", reciever);

    thread::sleep(Duration::from_millis(1000));

    info!("sending data now");
    let input_file = File::open(file_name)?;
    let file_len = input_file.metadata()?.len();
    let mut offset = 0;
    loop {
        let mut file_buf = [0u8;508];
        #[cfg(target_os = "linux")]
        {
            input_file.read_at(&mut file_buf, offset)?;
        }
        #[cfg(target_os = "windows")]
        {
            input_file.seek_read(&mut file_buf, offset)?;
        }

        sock.send_to(&file_buf, reciever).await?;

        offset += 508;
        if offset >= file_len {
            break;
        }
    }


    Ok(())
}
