use std::{
    error,
    error::Error,
    fmt,
    fs::{self, File},
    io::Write,
    net::SocketAddr,
    path::Path,
    process,
    str::FromStr,
    sync::Arc,
    time::Duration,
    vec,
};

#[cfg(target_os = "windows")]
use std::fs::create_dir_all;

#[cfg(target_os = "linux")]
use std::env;

use chrono::Local;
use clap::Parser;
use colored::Colorize;
use env_logger::Builder;
use log::{info, LevelFilter};
use rand::Rng;
use serde::{Deserialize, Serialize};
use shared::messages::{
    ip_for_code::IpForCode, recieving_ip::RecievingIp, you_have_file::YouHaveFile, Message,
    ServerMsg,
};
use tokio::{net::UdpSocket, time};

use crate::{recieving::reciever, sending::sender};

mod recieving;
mod sending;

fn get_config_filename() -> String {
    #[cfg(target_os = "linux")]
    let config_home = env::var("XDG_CONFIG_HOME")
        .or_else(|_| var("HOME").map(|home| format!("{}/.mork_config", home)))
        .to_string();
    #[cfg(target_os = "windows")]
    let config_home = format!(
        "C:\\Users\\{}\\AppData\\Roaming\\mork\\mork_config.toml",
        whoami::username()
    );

    config_home
}

// TODO: longer codes
// TODO: Check for enough disk space
// TODO: Function for getting new server list
// TODO: Function for updating program
// TODO: Do not store file size in server

#[derive(Serialize, Deserialize)]
struct Config {
    server_ips: Vec<String>,
}

impl Config {
    fn new() -> Self {
        Self {
            server_ips: vec!["92.244.2.150:47335".to_string()],
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
    input: Option<String>,

    #[clap(short, long)]
    code: Option<String>,

    #[clap(short, long)]
    output: Option<String>,
}

fn _get_msg_from_raw(raw: &[u8]) -> Result<ServerMsg, &'static str> {
    if let Ok(have_file) = YouHaveFile::from_raw(raw) {
        Ok(ServerMsg::YouHaveFile(have_file))
    } else if let Ok(i_have_code) = IpForCode::from_raw(raw) {
        Ok(ServerMsg::IpForCode(i_have_code))
    } else if let Ok(receving_ip) = RecievingIp::from_raw(raw) {
        Ok(ServerMsg::Recieving(receving_ip))
    } else {
        Err("could not make into any message")
    }
}

async fn punch_hole(sock: &UdpSocket, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    sock.send_to(&[255u8], addr).await?;

    Ok(())
}

fn get_config() -> Config {
    let config_filename = get_config_filename();
    // Check if settings file exists
    if !Path::new(&config_filename).exists() {
        let config = Config::new();

        let confing_str = toml::to_string(&config).unwrap();
        let mut file = match File::create(&config_filename) {
            Ok(file) => file,
            Err(_) => {
                #[cfg(target_os = "windows")]
                // Create directory for config
                {
                    create_dir_all(Path::new(&config_filename).parent().unwrap()).unwrap();
                }
                // Redo this
                return get_config();
            }
        };

        // Write a &str in the file (ignoring the result).
        write!(&mut file, "{}", confing_str).unwrap();
    }

    let contents = fs::read_to_string(&config_filename)
        .expect("Something went wrong reading the appdata file");

    let config: Config = toml::from_str(contents.as_str()).unwrap();

    config
}

fn ensure_global_ip(addr: SocketAddr, server_ip: &SocketAddr) -> SocketAddr {
    if ip_rfc::global(&addr.ip()) {
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

    match args.input {
        Some(input) => {
            sender(input, sock, server_addr, SendMethod::Index).await?;
        }
        None => {
            let code = match args.code {
                Some(code) => code,
                None => {
                    eprintln!("code or input file must be set");
                    process::exit(0);
                }
            };

            let send_method = SendMethod::Index;
            reciever(code, sock, server_addr, args.output, send_method).await?;
            info!("successfully recieved file");
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
