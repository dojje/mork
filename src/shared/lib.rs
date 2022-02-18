use std::net::SocketAddr;

pub mod messages;

pub struct ClientAddr {
    pub addr: SocketAddr,
    pub in_port: u16,
    pub out_port: u16,
}
