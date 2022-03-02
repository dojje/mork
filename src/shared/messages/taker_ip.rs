use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use super::Message;
use message_derive::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 3)]
pub struct TakerIp {
    pub ip: SocketAddr,
}

impl TakerIp {
    pub fn new(ip: SocketAddr) -> Self {
        Self { ip }
    }
}
