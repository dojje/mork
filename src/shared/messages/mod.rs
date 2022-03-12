// From sending client
pub mod have_file;

// From recieving client
pub mod i_have_code;

// From Server
pub mod ip_for_code;
pub mod recieving_ip;
pub mod you_have_file;

pub enum ClientMsg {
    HaveFile(have_file::HaveFile),
    IHaveCode(i_have_code::IHaveCode),
    None,
}

pub enum ServerMsg {
    YouHaveFile(you_have_file::YouHaveFile),
    IpForCode(ip_for_code::IpForCode),
    Recieving(recieving_ip::RecievingIp),
}
pub trait Message {
    fn to_raw(&self) -> Vec<u8>;
    fn from_raw(raw: &[u8]) -> Result<Self, &'static str>
    where
        Self: Sized;
}

// have_file: 0
// i_have_code: 1
// ip_for_code: 2
// recieving_ip: 3
// you_have_file: 4
// holepunch: 255
