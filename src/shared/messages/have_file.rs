use std::net::SocketAddr;

use message_derive::Message;
use serde::{Deserialize, Serialize};

use crate::ClientAddr;

use super::Message;

#[derive(Deserialize, Serialize, Message)]
#[message(msg_code = 0)]
pub struct HaveFile {
    pub in_port: u16,
    pub out_port: u16
}

impl HaveFile {
    pub fn to_addr(self, addr: SocketAddr) -> ClientAddr {
        ClientAddr {
            addr,
            in_port: self.in_port,
            out_port: self.out_port,
        }
    }
}
