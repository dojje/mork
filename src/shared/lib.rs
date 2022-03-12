use std::{error::Error, net::SocketAddr};

use messages::Message;
use tokio::net::UdpSocket;

pub mod messages;

#[derive(Clone)]
pub struct Transfer {
    pub file_haver: SocketAddr,
    pub file_name: String,
    pub file_len: u64,
}

impl Transfer {
    pub fn new(file_haver: SocketAddr, file_name: String, file_len: u64) -> Self {
        Self {
            file_haver,
            file_name,
            file_len,
        }
    }
}

pub async fn send_msg<T: Message>(
    sock: &UdpSocket,
    msg: &T,
    target: &SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let msg_raw = msg.to_raw();
    sock.send_to(msg_raw.as_slice(), target).await?;
    Ok(())
}
