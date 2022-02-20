use std::{io::Write, collections::HashMap, error::Error};
use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{LevelFilter, info};
use shared::{messages::{ClientMsg, i_have_code::IHaveCode, Message, have_file::HaveFile, you_have_file::YouHaveFile, ip_for_code::IpForCode}, Transfer};
use tokio::{net::UdpSocket};

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

fn new_code<T>(code_map: &HashMap<&'static str, T>) -> Result<&'static str, &'static str> {
    let code = "asdfhejemil";
    loop {
        if !code_map.contains_key(code) {
            return Ok(code);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Init logger
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

    // Make hashmap for every transfer
    let mut code_map: HashMap<&'static str, Transfer> = HashMap::new();

    // Create socket for recieving all messages
    let sock = UdpSocket::bind("0.0.0.0:47335").await?;

    info!("server ready");
    loop {
        // Recieve message
        let mut buf = [0u8;8192];
        let (amt, src) = sock.recv_from(&mut buf).await?;
        info!("got {} bytes from {:?}", amt, &src);
        let msg_buf = &buf[0..amt];

        info!("msg 1st is {}", msg_buf[0]);

        // TODO Spawn new thread for every message

        match get_msg_from_raw(msg_buf)?{
            ClientMsg::HaveFile(have_file) => {
                info!("msg was have_file msg");
                let code = new_code(&code_map)?;

                let resp = YouHaveFile::new(code.to_string());

                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;

                let transfer = Transfer::new(src, have_file.file_name);
                code_map.insert(code, transfer);
            },

            ClientMsg::IHaveCode(have_code) => {
                info!("msg was have_code msg");
                let transfer = match code_map.get(have_code.code.as_str()) {
                    Some(transfer) => transfer,
                    None => continue, // TODO Send error message to client
                };

                let resp = IpForCode::from_transfer(transfer.clone());
                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;

            
            },
        }

    }
}
