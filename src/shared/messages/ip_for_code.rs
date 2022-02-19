use std::net::SocketAddr;

use message_derive::Message;
use serde::{Serialize, Deserialize};

use super::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 2)]
pub struct IpForCode {
    pub ip: SocketAddr,
    pub give_in_port: u16,
    pub give_out_port: u16
}
