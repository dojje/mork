// From giver client
pub mod have_file;

// From taker client
pub mod i_have_code;

// From Server
pub mod you_have_file;
pub mod ip_for_code;
pub mod taker_ip;

pub enum ClientMsg {
    HaveFile(have_file::HaveFile),
    IHaveCode(i_have_code::IHaveCode),
}

pub enum ServerMsg {
    YouHaveFile(you_have_file::YouHaveFile),
    IpForCode(ip_for_code::IpForCode),
    TakerIp(taker_ip::TakerIp)
}

pub trait Message {
    fn to_raw(&self) -> Vec<u8>;
    fn from_raw(raw: &[u8]) -> Result<Self, &'static str> where Self: Sized;
}
