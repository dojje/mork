use std::net::SocketAddr;

pub struct IpForCode {
    pub ip: SocketAddr,
    pub give_in_port: u16,
    pub give_out_port: u16
}