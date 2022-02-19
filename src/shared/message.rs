
pub trait Message {
    fn to_raw(&self) -> Vec<u8>;
    fn from_raw(raw: &[u8]) -> Result<Self, &'static str> where Self: Sized;
}
