use std::net::SocketAddr;

use serde::{Serialize, Deserialize};

use message_derive::Message;
use super::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 3)]
pub struct TakerIp {
    pub ip: SocketAddr,
}

impl TakerIp {
    pub fn new(ip: SocketAddr) -> Self {
        Self {
            ip,
        }
    }
}
