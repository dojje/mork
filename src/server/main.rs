use std::{io::Write, collections::HashMap, error::Error, char};
use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{LevelFilter, info};
use rand::Rng;
use shared::{messages::{ClientMsg, i_have_code::IHaveCode, Message, have_file::HaveFile, you_have_file::YouHaveFile, ip_for_code::IpForCode, taker_ip::TakerIp}, Transfer};
use tokio::{net::UdpSocket};

const CODE_CHARS: &'static [char] = &[
    'A',
    'B',
    'C',
    'D',
    'E',
    'F',
    'G',
    'H',
    'I',
    'J',
    'K',
    'L',
    'M',
    'N',
    'O',
    'P',
    'Q',
    'R',
    'S',
    'T',
    'U',
    'V',
    'W',
    'X',
    'Y',
    'Z',
    '1',
    '2',
    '3',
    '4',
    '5',
    '6',
    '7',
    '8',
    '9',
    '0',
];

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
    let mut code_map: HashMap<String, Transfer> = HashMap::new();

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
                let transfer = match code_map.get(have_code.code.to_uppercase().as_str()) {
                    Some(transfer) => transfer,
                    None => continue, // TODO Send error message to client
                };

                // Send taker ip to giver
                let taker_ip = TakerIp::new(src);
                sock.send_to(&taker_ip.to_raw(), transfer.file_haver).await?;

                let resp = IpForCode::from_transfer(transfer.clone());
                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;
            },
        }

    }
}
