use std::net::SocketAddr;

use message_derive::Message;
use serde::{Serialize, Deserialize};

use crate::Transfer;

use super::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 2)]
pub struct IpForCode {
    pub ip: SocketAddr,
    pub file_name: String,
    pub file_len: u64
}

impl IpForCode {
    pub fn from_transfer(transfer: Transfer) -> Self {
        Self {
            ip: transfer.file_haver,
            file_name: transfer.file_name,
            file_len: transfer.file_len

        }
    }
}
