use std::{net::SocketAddr, error::Error, fs::File, io::Write, sync::Arc, };

use log::info;
use shared::{messages::{i_have_code::IHaveCode, ip_for_code::IpForCode, Message}, send_msg};
use tokio::net::UdpSocket;

use crate::{punch_hole, recv, ensure_global_ip};


pub async fn reciever(code: String, sock: Arc<UdpSocket>, server_addr: SocketAddr, output: Option<String>) -> Result<(), Box<dyn Error>> {
    // Send message to server
    let i_have_code = IHaveCode::new(code);
    send_msg(&sock, &i_have_code, server_addr).await?;

    let msg_buf = recv(&sock, server_addr).await?;

    let ip_for_code = IpForCode::from_raw(msg_buf.as_slice())?;
    let ip = ensure_global_ip(ip_for_code.ip, &server_addr);
    info!("file name: {}", &ip_for_code.file_name);
    info!("other ip: {}", &ip);

    // Punch hole
    punch_hole(&sock, ip).await?;
    info!("ready to recieve");

    let filename = match output {
        Some(filename) => filename,
        None => ip_for_code.file_name,
    };
    
    let mut file = File::create(filename).unwrap();

    let mut msg_num: u64 = 0;
    loop {
        let msg_buf = recv(&sock, ip).await?;

        if msg_num == 0 && msg_buf.len() == 1 && msg_buf[0] == 255 {
            // Skip if the first iteration is a hole punch msg
            continue;
        }

        let num_bytes: [u8; 8] = [
            msg_buf[0],
            msg_buf[1],
            msg_buf[2],
            msg_buf[3],
            msg_buf[4],
            msg_buf[5],
            msg_buf[6],
            msg_buf[7]
        ];

        let _giver_msg_num = u64::from_be_bytes(num_bytes);
        let rest = &msg_buf[8..];

        file.write(&rest).unwrap();
        msg_num += 1;
    }

    // Ok(())
}
