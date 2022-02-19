use std::{io::Write, collections::HashMap, sync::mpsc::Sender, net::SocketAddr, error::Error, mem::size_of_val};
use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{LevelFilter, info};
use shared::{messages::{ClientMsg, i_have_code::IHaveCode, Message, have_file::HaveFile, you_have_file::YouHaveFile}, ClientAddr};
use tokio::{net::UdpSocket, sync::mpsc::channel};

fn get_msg_from_raw(raw: &[u8]) -> Result<ClientMsg, &'static str> {
    if let Ok(have_file) = HaveFile::from_raw(raw) {
        Ok(ClientMsg::HaveFile(have_file))
    }
    else if let Ok(i_have_code) = IHaveCode::from_raw(raw) {
        Ok(ClientMsg::IHaveCode(i_have_code))
    }

    else {
        Err("could not make into any message")
    }
}

fn new_code(code_map: &HashMap<&'static str, ClientAddr>) -> Result<&'static str, &'static str> {
    let code = "asdf";
    loop {
        if !code_map.contains_key(code) {
            return Ok(code);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

        let msg = get_msg_from_raw(msg_buf)?;
        match msg {
            ClientMsg::HaveFile(have_file) => {
                let addr = have_file.to_addr(src);
                let code = new_code(&code_map)?;

                let resp = YouHaveFile::new(code.to_string());

            },
            ClientMsg::IHaveCode(_) => todo!(),
        }

    }
}
