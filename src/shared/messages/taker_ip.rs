use std::net::SocketAddr;

use serde::{Serialize, Deserialize};

use message_derive::Message;
use super::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 3)]
pub struct TakerIp {
    pub ip: SocketAddr,
    pub taker_in: u16,
    pub taker_out: u16
}