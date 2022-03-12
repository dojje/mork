use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use super::Message;
use message_derive::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 3)]
pub struct RecievingIp {
    pub ip: SocketAddr,
}

impl RecievingIp {
    pub fn new(ip: SocketAddr) -> Self {
        Self { ip }
    }
}
