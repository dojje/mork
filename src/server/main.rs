use chrono::Local;
use colored::Colorize;
use env_logger::Builder;
use log::{info, LevelFilter};
use rand::Rng;
use shared::{
    messages::{
        have_file::HaveFile, i_have_code::IHaveCode, ip_for_code::IpForCode,
        recieving_ip::RecievingIp, you_have_file::YouHaveFile, ClientMsg, Message,
    },
    Transfer,
};
use std::{char, collections::HashMap, error::Error, io::Write, net::SocketAddr};
use tokio::net::UdpSocket;

const CODE_CHARS: &'static [char] = &[
    // Am not using O and 0 because of confusion
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T',
    'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

fn get_msg_from_raw(raw: &[u8]) -> Result<ClientMsg, &'static str> {
    if let Ok(have_file) = HaveFile::from_raw(raw) {
        Ok(ClientMsg::HaveFile(have_file))
    } else if let Ok(i_have_code) = IHaveCode::from_raw(raw) {
        Ok(ClientMsg::IHaveCode(i_have_code))
    } else if raw == &[255u8] {
        Ok(ClientMsg::HolePunch)
    } else {
        Ok(ClientMsg::None)
    }
}

fn new_code(code_map: &HashMap<String, Transfer>) -> Result<String, Box<dyn Error>> {
    loop {
        // Generate code
        let mut rng = rand::thread_rng();
        let mut code = String::new();
        for _ in 0..=4 {
            let char_i = rng.gen_range(0..CODE_CHARS.len());
            code.push(CODE_CHARS[char_i]);
        }

        // use code
        if !code_map.contains_key(&code) || code_map.get(&code).unwrap().has_expired() {
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
    let mut addr_map: HashMap<SocketAddr, String> = HashMap::new(); // Addr map for keeping track of all sending clients

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
                let code = new_code(&mut code_map)?;

                let resp = YouHaveFile::new(code.to_string());

                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;

                info!(
                    "client from {} is ready to send {} with code {}",
                    src, have_file.file_name, code
                );
                let transfer = Transfer::new(src, have_file.file_name, have_file.file_len);
                code_map.insert(code.clone(), transfer);
                addr_map.insert(src, code);
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

                // Send recieving ip to sending client
                let recieving_ip = RecievingIp::new(src);
                sock.send_to(&recieving_ip.to_raw(), transfer.file_haver)
                    .await?;

                let resp = IpForCode::from_transfer(transfer.clone());
                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;
            }
            ClientMsg::None => {}
            ClientMsg::HolePunch => {
                let code = addr_map.get(&src);
                let transfer = code_map.get_mut(code.unwrap()).unwrap();

                transfer.update();
            }
        }
    }
}
