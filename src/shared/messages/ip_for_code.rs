use std::net::SocketAddr;

use mork_message_derive::Message;
use serde::{Deserialize, Serialize};

use crate::Transfer;

use super::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 2)]
pub struct IpForCode {
    pub ip: SocketAddr,
    pub file_name: String,
}

impl IpForCode {
    pub fn from_transfer(transfer: Transfer) -> Self {
        Self {
            ip: transfer.file_haver,
            file_name: transfer.file_name,
        }
    }
}
