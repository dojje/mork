use serde::{Serialize, Deserialize};

use super::Message;

#[derive(Serialize, Deserialize, Clone)]
pub struct IHaveCode {
    pub code: String,
    pub in_port: u16,
    pub out_port: u16,
}

impl Message for IHaveCode {
    fn to_raw(&self) -> Vec<u8> {
        let mut og = bincode::serialize(&self).unwrap(); 

        let mut have_file = vec![0];
        have_file.append(&mut og);
        
        have_file
    }

    fn from_raw(raw: &[u8]) -> Result<Self, &'static str> where Self: Sized {
        if !raw[0] == 1 {
            return Err("not good msg");
        }

        let have_file: Self = match bincode::deserialize(&raw) {
            Ok(v) => v,
            Err(_) => return Err("deserialising failed"),
        };
        

        Ok(have_file)
    }
}
