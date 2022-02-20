use std::{net::{SocketAddr}, error::Error};

use log::info;
use rand::{Rng};
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
    /// Subcommand to execute
    #[clap(subcommand)]
    action: Action,

    /// Code for file to recieve
    /// Must be set if it should recieve files
    #[clap(short, long)]
    code: Option<String>,
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

async fn punch_hole(sock: &UdpSocket, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    sock.send_to(&[254u8], addr).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let args = Args::parse();
    let mut rng = rand::thread_rng();

    let port: u16 = rng.gen_range(8192..u16::MAX);
    info!("using port {}", port);

    let server_addr = SocketAddr::from(([127,0,0,1], 47335));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let sock = UdpSocket::bind(addr).await?;
    
    match args.action {
        Action::Give => {
            sender("hey_guys.txt", sock, server_addr).await?;
        },
        Action::Take => {
            let code = match args.code {
                Some(code) => code,
                None => {panic!("code must be set");},
            };
            
            reciever(code, sock, server_addr).await?;
        },
    }

    Ok(())
}

async fn recv(sock: &UdpSocket) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = [0u8;8192];
    let (amt, _src) = sock.recv_from(&mut buf).await?;
    let msg_buf = &buf[0..amt];

    Ok(msg_buf.to_owned())
}

async fn reciever(code: String, sock: UdpSocket, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    // Send message to server
    let i_have_code = IHaveCode::new(code);
    send_msg(&sock, i_have_code, server_addr).await?;

    let msg_buf = recv(&sock).await?;

    let ip_for_code = IpForCode::from_raw(msg_buf.as_slice())?;
    println!("file name: {}", &ip_for_code.file_name);
    println!("other ip: {}", &ip_for_code.ip);

    punch_hole(&sock, ip_for_code.ip).await?;
    
    loop {
        let msg_buf = recv(&sock).await?;
        println!("msg 0th: {}", msg_buf[0]);
    }

    // Ok(())
}

async fn sender(file_name: &'static str, sock: UdpSocket, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let have_file = HaveFile::new(file_name.to_string());

    send_msg(&sock, have_file, server_addr).await?;
    
    // TODO Send this once a second until it gets answer from server

    let msg_buf = recv(&sock).await?;

    // TODO Check where the server message is comming from

    println!("you have file 0th: {}", msg_buf[0]);
    let you_have_file = YouHaveFile::from_raw(msg_buf.as_slice())?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    let msg_buf = recv(&sock).await?;
    println!("msg 0th: {}", msg_buf[0]);
    let file_reciever = TakerIp::from_raw(msg_buf.as_slice())?;

    println!("reciever ip: {}", file_reciever.ip);

    sock.send_to(&[9, 2, 3, 4], file_reciever.ip).await?;

    Ok(())
}
