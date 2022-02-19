use std::net::SocketAddr;

pub mod messages;

#[derive(Clone)]
pub struct Transfer {
    file_haver: SocketAddr,
    file_name: String,
}

impl Transfer {
    pub fn new(file_haver: SocketAddr, file_name: String) -> Self {
        Self {
            file_haver,
            file_name,
        }
    }
}
