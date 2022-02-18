use std::net::SocketAddr;

pub mod messages;

pub struct ClientAddr {
    pub addr: SocketAddr,
    pub port_in: u16,
    pub port_out: u16,
}
