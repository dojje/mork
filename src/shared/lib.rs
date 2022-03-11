use std::{error::Error, net::SocketAddr};

use messages::Message;
use tokio::net::UdpSocket;

pub mod messages;

#[cfg(feature = "sim_wan")]
use std::error;

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

#[cfg(feature = "sim_wan")]
pub async fn send_maybe(
    sock: &UdpSocket,
    buf: &[u8],
    reciever: &SocketAddr,
) -> Result<(), Box<dyn error::Error>> {
    {
        let num = rand::random::<u8>();

        if num <= 127 {
            sock.send_to(&buf, reciever).await?;
        } else {
        }
    }

    Ok(())
}

pub async fn send_msg<T: Message>(
    sock: &UdpSocket,
    msg: &T,
    target: &SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let msg_raw = msg.to_raw();
    #[cfg(feature = "sim_wan")]
    send_maybe(&sock, msg_raw.as_slice(), target).await?;
    #[cfg(not(feature = "sim_wan"))]
    sock.send_to(msg_raw.as_slice(), target).await?;
    Ok(())
}
