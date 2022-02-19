use std::{net::{SocketAddr}, error::Error};

use shared::{messages::{have_file::HaveFile, you_have_file::{self, YouHaveFile}, ServerMsg, Message, ip_for_code::IpForCode, taker_ip::TakerIp}, send_msg};
use tokio::net::UdpSocket;

fn get_msg_from_raw(raw: &[u8]) -> Result<ServerMsg, &'static str> {
    if let Ok(have_file) = YouHaveFile::from_raw(raw) {
        Ok(ServerMsg::YouHaveFile(have_file))
    }
    else if let Ok(i_have_code) = IpForCode::from_raw(raw) {
        Ok(ServerMsg::IpForCode(i_have_code))
    }
    else if let Ok(taker_ip) = TakerIp::from_raw(raw) {
        Ok(ServerMsg::TakerIp(taker_ip))
    }

    else {
        Err("could not make into any message")
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let port: u16 = 46352;
    let server_addr = SocketAddr::from(([127,0,0,1], 47335));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let sock = UdpSocket::bind(addr).await?;

    sender("hey_guys.txt", sock, server_addr).await
}

async fn sender(file_name: &'static str, sock: UdpSocket, target: SocketAddr) -> Result<(), Box<dyn Error>>{
    let have_file = HaveFile::new(file_name.to_string());

    send_msg(&sock, have_file, target).await?;
    
    // TODO Send this once a second until it gets answer from server
    
    let mut buf = [0u8;8192];
    let (amt, src) = sock.recv_from(&mut buf).await?;
    let msg_buf = &buf[0..amt];

    let you_have_file = YouHaveFile::from_raw(msg_buf)?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    let mut buf = [0u8;8192];
    let (amt, _) = sock.recv_from(&mut buf).await?;
    let msg_buf = &buf[0..amt];

    let file_reciever = TakerIp::from_raw(msg_buf)?;


    Ok(())
}
