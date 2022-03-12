use std::{
    error::Error,
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use messages::Message;
use tokio::net::UdpSocket;

pub mod messages;

#[derive(Clone)]
pub struct Transfer {
    pub file_haver: SocketAddr,
    pub file_name: String,
    pub file_len: u64,
    pub last_updated: u64,
}

impl Transfer {
    pub fn new(file_haver: SocketAddr, file_name: String, file_len: u64) -> Self {
        let last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            file_haver,
            file_name,
            file_len,
            last_updated,
        }
    }

    pub fn update(&mut self) {
        let last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.last_updated = last_updated;
    }

    pub fn has_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.last_updated + 10 < now
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
