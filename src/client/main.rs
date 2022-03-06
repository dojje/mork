#![feature(ip)]
use std::{
    error,
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Write},
    net::SocketAddr,
    path::Path,
    process,
    str::FromStr,
    sync::Arc,
    time::Duration,
    vec,
};

use chrono::Local;
use clap::Parser;
use colored::Colorize;
use env_logger::Builder;
use log::{info, LevelFilter};
use rand::Rng;
use serde::{Deserialize, Serialize};
use shared::messages::{
    ip_for_code::IpForCode, taker_ip::TakerIp, you_have_file::YouHaveFile, Message, ServerMsg,
};
use tokio::{net::UdpSocket, time};

#[cfg(target_os = "linux")]
use std::os::unix::fs::FileExt;
#[cfg(target_os = "windows")]
use std::os::windows::prelude::FileExt;

use crate::{giver::sender, taker::reciever};

mod giver;
mod taker;

#[cfg(feature = "sim_wan")]
use shared::send_maybe;

const CONFIG_FILENAME: &'static str = "filesender_data.toml";
// TODO longer codes
// TODO fix clap order, make it so that you can use any order
// TODO Check for enough disk space
// TODO Function for getting new server list
// TODO Function for updating program

fn read_position(file: &File, buf: &mut [u8], offset: u64) -> Result<usize, Box<dyn error::Error>> {
    #[cfg(target_os = "linux")]
    let amt = file.read_at(buf, offset)?;

    #[cfg(target_os = "windows")]
    let amt = file.seek_read(buf, offset)?;

    Ok(amt)
}

fn write_position(file: &File, buf: &[u8], offset: u64) -> Result<usize, Box<dyn error::Error>> {
    #[cfg(target_os = "linux")]
    let amt = file.write_at(&buf, offset)?;

    #[cfg(target_os = "windows")]
    let amt = file.seek_write(&buf, offset)?;

    Ok(amt)
}

#[derive(Serialize, Deserialize)]
struct Config {
    server_ips: Vec<String>,
}

impl Config {
    fn new() -> Self {
        Self {
            server_ips: vec!["127.0.0.1:47335".to_string()],
        }
    }
}

#[derive(Clone)]
pub enum SendMethod {
    Burst,   // Send packets without any sort of check
    Confirm, // The reciever needs to confirm that a packet has been sent
    Index,   // The sender sends the index of the packet, it get's placed where it belongs
}

// Things for clap

#[derive(clap::Subcommand, Debug)]
enum Action {
    Give,
    Take,
}

// TODO Set custom port
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

    #[clap(short, long, default_value = "seq")]
    recv_mode: String,
}

fn _get_msg_from_raw(raw: &[u8]) -> Result<ServerMsg, &'static str> {
    if let Ok(have_file) = YouHaveFile::from_raw(raw) {
        Ok(ServerMsg::YouHaveFile(have_file))
    } else if let Ok(i_have_code) = IpForCode::from_raw(raw) {
        Ok(ServerMsg::IpForCode(i_have_code))
    } else if let Ok(taker_ip) = TakerIp::from_raw(raw) {
        Ok(ServerMsg::TakerIp(taker_ip))
    } else {
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

    let contents =
        fs::read_to_string(CONFIG_FILENAME).expect("Something went wrong reading the appdata file");

    let config: Config = toml::from_str(contents.as_str()).unwrap();

    config
}

fn ensure_global_ip(addr: SocketAddr, server_ip: &SocketAddr) -> SocketAddr {
    if addr.ip().is_global() {
        return addr;
    }

    // If address is not global
    // Then the address is probalby from the servers lan
    // This could happen if the user is running their own server
    // Use the servers global ip and the clients port
    SocketAddr::new(server_ip.ip(), addr.port())
}

#[derive(Debug, Clone)]
struct NotRightAmountError;

impl fmt::Display for NotRightAmountError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid first item to double")
    }
}

fn u8s_to_u64(nums: &[u8]) -> io::Result<u64> {
    if nums.len() != 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "nums must be 8 bytes long",
        ));
    }
    let msg_u8: [u8; 8] = [
        nums[0], nums[1], nums[2], nums[3], nums[4], nums[5], nums[6], nums[7],
    ];

    let big_number = u64::from_be_bytes(msg_u8);
    Ok(big_number)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // init log
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level().to_string().blue(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        .init();

    // Read arguemts
    let args = Args::parse();
    #[cfg(feature = "sim_wan")]
    info!("simulating wide area network");

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
            sender(input, sock, server_addr, SendMethod::Index).await?;
        }
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
                }
            };

            let send_method = SendMethod::Index;
            reciever(code, sock, server_addr, args.output, send_method).await?;
        }
    }

    Ok(())
}

async fn recv(
    sock: &UdpSocket,
    from: &SocketAddr,
    buf: &mut [u8],
) -> Result<usize, Box<dyn Error>> {
    loop {
        let (amt, src) = sock.recv_from(buf).await?;

        if &src == from {
            return Ok(amt);
        }
    }
}

async fn send_unil_recv(
    sock: &UdpSocket,
    msg: &[u8],
    addr: &SocketAddr,
    buf: &mut [u8],
    interval: u64,
) -> Result<usize, Box<dyn error::Error>> {
    let mut send_interval = time::interval(Duration::from_millis(interval));
    let amt = loop {
        tokio::select! {
            _ = send_interval.tick() => {
                #[cfg(feature = "sim_wan")]
                send_maybe(&sock, msg, addr).await?;
                #[cfg(not(feature = "sim_wan"))]
                sock.send_to(msg, addr).await?;

            }

            result = sock.recv_from(buf) => {
                let (amt, src) = result?;
                if &src != addr {
                    continue;
                }
                break amt;
            }
        }
    };

    Ok(amt)
}
