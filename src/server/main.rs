use std::{io::Write, io, collections::HashMap, sync::mpsc::Sender, net::SocketAddr};
use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{LevelFilter, info};
use shared::ClientAddr;
use tokio::{net::UdpSocket, sync::mpsc::channel};

#[tokio::main]
async fn main() -> io::Result<()> {
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level().to_string().blue(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Trace)
        .init();

    let code_map: HashMap<&'static str, ClientAddr> = HashMap::new();

    // Create socket for recieving all messages
    let sock = UdpSocket::bind("0.0.0.0:47335").await?;

    info!("server ready");
    loop {
        let mut buf = [0u8;8192];
        let (amt, src) = sock.recv_from(&mut buf).await?;

        let msg_buf = &buf[0..amt];

        

    }
}
