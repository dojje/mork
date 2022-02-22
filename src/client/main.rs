#![feature(ip)]
use std::{net::{SocketAddr}, error::Error, fs::{File, self}, io::Write, path::Path, vec, str::FromStr, process, sync::Arc};

use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{info, LevelFilter};
use rand::{Rng};
use serde::{Serialize, Deserialize};
use shared::{messages::{you_have_file::{YouHaveFile}, ServerMsg, Message, ip_for_code::IpForCode, taker_ip::TakerIp, }};
use tokio::net::UdpSocket;
use clap::Parser;

use crate::{giver::sender, taker::reciever};

mod giver;
mod taker;

const CONFIG_FILENAME: &'static str = "filesender_data.toml";
// TODO longer codes
// TODO fix clap order, make it so that you can use any order

#[derive(Serialize, Deserialize)]
struct Config {
    server_ips: Vec<String>
}

impl Config {
    fn new() -> Self {
        Self {
            server_ips: vec!["127.0.0.1:47335".to_string()]
        }
    }
}

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

    #[clap(short, long)]
    input: Option<String>,

    #[clap(short, long)]
    output: Option<String>,
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
    sock.send_to(&[255u8], addr).await?;

    Ok(())
}

fn get_config() -> Config {
    // Check if settings file exists
    if !Path::new(CONFIG_FILENAME).exists() {
        let config = Config::new();

        let confing_str = toml::to_string(&config).unwrap();
        let mut file = File::create(CONFIG_FILENAME).unwrap();

        // Write a &str in the file (ignoring the result).
        write!(&mut file, "{}", confing_str).unwrap();
    }

    let contents = fs::read_to_string(CONFIG_FILENAME)
        .expect("Something went wrong reading the appdata file");

    let config: Config = toml::from_str(contents.as_str()).unwrap();

    config
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // init log
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level().to_string().blue(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    // Read arguemts
    let args = Args::parse();

    // Read from config
    let config = get_config();
    let server_addr = SocketAddr::from_str(config.server_ips[0].as_str())?;
    info!("server ip is {}", server_addr);


    // Come up with port
    let mut rng = rand::thread_rng();
    let port: u16 = rng.gen_range(8192..u16::MAX);
    info!("using port {}", port);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("binding to addr");
    let sock = Arc::new(UdpSocket::bind(addr).await?);
    
    match (args.action, args.input) {
        (Action::Give, Some(input)) => {
            sender(input, sock, server_addr).await?;
        },
        (Action::Give, None) => {
            eprintln!("input file not set");
            process::exit(0);
        }
        (Action::Take, _) => {
            let code = match args.code {
                Some(code) => code,
                None => {
                    eprintln!("code must be set");
                    process::exit(0);
                },
            };
            reciever(code, sock, server_addr, args.output).await?;
        },
    }

    Ok(())
}

async fn recv(sock: &UdpSocket, from: SocketAddr) -> Result<Vec<u8>, Box<dyn Error>> {
    loop {
        let mut buf = [0u8;8192];
        let (amt, src) = sock.recv_from(&mut buf).await?;

        if src == from {
            let msg_buf = &buf[0..amt];
            return Ok(msg_buf.to_owned());
        }
    }
}
