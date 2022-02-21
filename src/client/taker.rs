use std::{net::SocketAddr, error::Error, fs::File, io::Write};

use log::info;
use shared::{messages::{i_have_code::IHaveCode, ip_for_code::IpForCode, Message}, send_msg};
use tokio::net::UdpSocket;

use crate::{punch_hole, recv};


pub async fn reciever(code: String, sock: UdpSocket, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    // Send message to server
    let i_have_code = IHaveCode::new(code);
    send_msg(&sock, &i_have_code, server_addr).await?;

    let msg_buf = recv(&sock, server_addr).await?;

    let ip_for_code = IpForCode::from_raw(msg_buf.as_slice())?;
    println!("file name: {}", &ip_for_code.file_name);
    println!("other ip: {}", &ip_for_code.ip);

    punch_hole(&sock, ip_for_code.ip).await?;
    info!("punched hoel to {}", ip_for_code.ip);
    
    let mut file = File::create(ip_for_code.file_name).unwrap();

    loop {
        info!("awaiting packet...");
        let msg_buf = recv(&sock, ip_for_code.ip).await?;

        file.write(&msg_buf.as_slice()).unwrap();
        info!("got packet!");
    }

    // Ok(())
}
