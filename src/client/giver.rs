use std::{net::SocketAddr, error::Error, time::Duration, thread};

use log::info;
use shared::{messages::{have_file::HaveFile, you_have_file::YouHaveFile, taker_ip::TakerIp, Message}, send_msg};
use tokio::net::UdpSocket;

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

    // TODO Keep the hole punched

    let msg_buf = recv(&sock, server_addr).await?;

    println!("msg 0th: {}", msg_buf[0]);
    let file_reciever = TakerIp::from_raw(msg_buf.as_slice())?;

    println!("reciever ip: {}", file_reciever.ip);

    punch_hole(&sock, file_reciever.ip).await?;
    info!("punched hole to {}", file_reciever.ip);

    thread::sleep(Duration::from_millis(1000));

    info!("sending data now");
    sock.send_to(&[0xCB, 0xCB, 65, 65], file_reciever.ip).await?;

    Ok(())
}
