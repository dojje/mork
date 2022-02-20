use std::{net::SocketAddr, error::Error, time::Duration, thread};

use log::info;
use shared::{messages::{have_file::HaveFile, you_have_file::YouHaveFile, taker_ip::TakerIp, Message}, send_msg};
use tokio::{net::UdpSocket, time};

use crate::{recv, punch_hole};


pub async fn sender(file_name: String, sock: UdpSocket, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let have_file = HaveFile::new(file_name);
    info!("contacting server");
    send_msg(&sock, have_file, server_addr).await?;
    
    // TODO Send this once a second until it gets answer from server

    let msg_buf = recv(&sock, server_addr).await?;

    println!("you have file 0th: {}", msg_buf[0]);
    let you_have_file = YouHaveFile::from_raw(msg_buf.as_slice())?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    let mut interval = time::interval(Duration::from_millis(500));

    let taker_ip = loop {
        tokio::select! {
            _ = interval.tick() => {
                // keep hole punched to server
                sock.send_to(&[255u8], server_addr).await?;
            }
            
            result = recv(&sock, server_addr) => {
                let msg_buf = result?;
                let taker_ip = TakerIp::from_raw(msg_buf.as_slice())?;
                break taker_ip;
            }
        }
    };

    println!("reciever ip: {}", taker_ip.ip);

    punch_hole(&sock, taker_ip.ip).await?;
    info!("punched hole to {}", taker_ip.ip);

    thread::sleep(Duration::from_millis(1000));

    info!("sending data now");
    sock.send_to(&[0xCB, 0xCB, 65, 65], taker_ip.ip).await?;

    Ok(())
}
