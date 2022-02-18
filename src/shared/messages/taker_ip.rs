use std::net::SocketAddr;

pub struct TakerIp {
    pub ip: SocketAddr,
    pub taker_in: u16,
    pub taker_out: u16
}