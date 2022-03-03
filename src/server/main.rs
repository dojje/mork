use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{info, LevelFilter};
use rand::Rng;
use shared::{
    messages::{
        have_file::HaveFile, i_have_code::IHaveCode, ip_for_code::IpForCode, taker_ip::TakerIp,
        you_have_file::YouHaveFile, ClientMsg, Message,
    },
    Transfer,
};
use std::{char, collections::HashMap, error::Error, io::Write};
use tokio::net::UdpSocket;

const CODE_CHARS: &'static [char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
];

fn get_msg_from_raw(raw: &[u8]) -> Result<ClientMsg, &'static str> {
    if let Ok(have_file) = HaveFile::from_raw(raw) {
        Ok(ClientMsg::HaveFile(have_file))
    } else if let Ok(i_have_code) = IHaveCode::from_raw(raw) {
        Ok(ClientMsg::IHaveCode(i_have_code))
    } else if raw == &[0u8] {
        Ok(ClientMsg::None)
    } else {
        Err("could not make into any message")
    }
}

fn new_code<T>(code_map: &HashMap<String, T>) -> Result<String, Box<dyn Error>> {
    let mut rng = rand::thread_rng();
    let mut code = String::new();

    for _ in 0..=4 {
        let char_i = rng.gen_range(0..CODE_CHARS.len());
        code.push(CODE_CHARS[char_i]);
    }

    loop {
        if !code_map.contains_key(&code) {
            return Ok(code);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO Set custom port
    // Init logger
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

    // Make hashmap for every transfer
    let mut code_map: HashMap<String, Transfer> = HashMap::new();

    // Create socket for recieving all messages
    let sock = UdpSocket::bind("0.0.0.0:47335").await?;

    info!("server ready");
    loop {
        // Recieve message
        let mut buf = [0u8; 8192];
        let (amt, src) = sock.recv_from(&mut buf).await?;
        let msg_buf = &buf[0..amt];

        let msg_res = get_msg_from_raw(msg_buf);
        let msg = match msg_res {
            Ok(msg) => msg,
            Err(_) => continue,
        };

        match msg {
            ClientMsg::HaveFile(have_file) => {
                let code = new_code(&code_map)?;

                let resp = YouHaveFile::new(code.to_string());

                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;

                info!(
                    "client from {} is ready to send {} with code {}",
                    src, have_file.file_name, code
                );
                let transfer = Transfer::new(src, have_file.file_name, have_file.file_len);
                code_map.insert(code, transfer);
            }

            ClientMsg::IHaveCode(have_code) => {
                let transfer = match code_map.get(have_code.code.to_uppercase().as_str()) {
                    Some(transfer) => transfer,
                    None => continue,
                };
                info!(
                    "client from {} want to recieve from code {}",
                    src, have_code.code
                );

                // Send taker ip to giver
                let taker_ip = TakerIp::new(src);
                sock.send_to(&taker_ip.to_raw(), transfer.file_haver)
                    .await?;

                let resp = IpForCode::from_transfer(transfer.clone());
                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;
            }
            ClientMsg::None => {}
        }
    }
}
