use std::{net::{SocketAddr}, error::Error};

use shared::{messages::{have_file::HaveFile, you_have_file::{YouHaveFile}, ServerMsg, Message, ip_for_code::IpForCode, taker_ip::TakerIp, i_have_code::{IHaveCode}}, send_msg};
use tokio::net::UdpSocket;
use clap::Parser;

#[derive(clap::Subcommand, Debug)]
enum Action {
    Give,
    Take
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(subcommand)]
    action: Action,

    /// Number of times to greet
    #[clap(short, long, default_value_t = 1)]
    count: u8,
}

fn _get_msg_from_raw(raw: &[u8]) -> Result<ServerMsg, &'static str> {
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
    let args = Args::parse();
    let port: u16 = 46352;
    let server_addr = SocketAddr::from(([127,0,0,1], 47335));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let sock = UdpSocket::bind(addr).await?;
    

    match args.action {
        Action::Give => {
            sender("hey_guys.txt", sock, server_addr).await?;
        },
        Action::Take => {
            reciever("cbcb", sock, server_addr).await?;
        },
    }

    Ok(())
}

async fn reciever(code: &'static str, sock: UdpSocket, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    // Send message to server
    let i_have_code = IHaveCode::new(code.to_string());
    send_msg(&sock, i_have_code, server_addr).await?;

    let mut buf = [0u8;8192];
    let (amt, _src) = sock.recv_from(&mut buf).await?;
    let msg_buf = &buf[0..amt];

    let ip_for_code = IpForCode::from_raw(msg_buf)?;
    println!("file name: {}", &ip_for_code.file_name);
    println!("other ip: {}", &ip_for_code.ip);

    

    Ok(())
}

async fn sender(file_name: &'static str, sock: UdpSocket, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let have_file = HaveFile::new(file_name.to_string());

    send_msg(&sock, have_file, server_addr).await?;
    
    // TODO Send this once a second until it gets answer from server

    let mut buf = [0u8;8192];
    let (amt, _src) = sock.recv_from(&mut buf).await?;
    let msg_buf = &buf[0..amt];

    // TODO Check where the server message is comming from

    let you_have_file = YouHaveFile::from_raw(msg_buf)?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    let mut buf = [0u8;8192];
    let (amt, _) = sock.recv_from(&mut buf).await?;
    let msg_buf = &buf[0..amt];

    let file_reciever = TakerIp::from_raw(msg_buf)?;

    println!("reciever ip: {}", file_reciever.ip);
    Ok(())
}
