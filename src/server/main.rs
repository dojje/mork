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
use std::{
    char, collections::HashMap, error::Error, io::Write, net::SocketAddr, sync::Arc, time::Duration,
};
use tokio::{net::UdpSocket, sync::Mutex, time};

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

async fn new_code(
    code_map: &Arc<Mutex<HashMap<String, Transfer>>>,
) -> Result<String, Box<dyn Error>> {
    loop {
        // Generate code
        let mut rng = rand::thread_rng();
        let mut code = String::new();
        for _ in 0..=4 {
            let char_i = rng.gen_range(0..CODE_CHARS.len());
            code.push(CODE_CHARS[char_i]);
        }

        // use code
        let code_map = code_map.lock().await;
        if !code_map.contains_key(&code) || code_map.get(&code).unwrap().has_expired() {
            return Ok(code);
        }
    }
}

async fn remove_expired(code_map: &Arc<Mutex<HashMap<String, Transfer>>>) {
    let mut code_map = code_map.lock().await;

    code_map.retain(|code, v| {
        if v.has_expired() {
            info!("removing code {}", code);
            false
        } else {
            true
        }
    });
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
    let code_map: Arc<Mutex<HashMap<String, Transfer>>> = Arc::new(Mutex::new(HashMap::new()));
    let addr_map: Arc<Mutex<HashMap<SocketAddr, String>>> = Arc::new(Mutex::new(HashMap::new())); // Addr map for keeping track of all sending clients

    let code_map_ = code_map.clone();
    tokio::spawn(async move {
        let code_map = code_map_;

        loop {
            remove_expired(&code_map).await;

            time::sleep(Duration::from_secs(60)).await;
        }
    });

    // Create socket for recieving all messages
    let sock = UdpSocket::bind("0.0.0.0:47335").await?;

    info!("server ready on port 47335");
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
                let code = new_code(&code_map).await?;

                let resp = YouHaveFile::new(code.to_string());

                let resp_raw = resp.to_raw();
                sock.send_to(resp_raw.as_slice(), src).await?;

                info!(
                    "client from {} is ready to send {} with code {}",
                    src, have_file.file_name, code
                );
                let transfer = Transfer::new(src, have_file.file_name, have_file.file_len);
                code_map.lock().await.insert(code.clone(), transfer);
                addr_map.lock().await.insert(src, code);
            }

            ClientMsg::IHaveCode(have_code) => {
                let code_map_ = code_map.lock().await;
                let transfer = match code_map_.get(have_code.code.to_uppercase().as_str()) {
                    Some(transfer) => transfer.clone(),
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
                let addr_map = addr_map.lock().await;
                let code = addr_map.get(&src).unwrap();
                code_map.lock().await.get_mut(code).unwrap().update();
            }
        }
    }
}
