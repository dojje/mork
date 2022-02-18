use std::net::SocketAddr;

pub mod messages;

pub struct ClientAddr {
    addr: SocketAddr,
    port_in: u16,
    port_out: u16,
}
