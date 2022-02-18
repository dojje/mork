use serde::{Deserialize, Serialize};

use super::Message;

#[derive(Deserialize, Serialize)]
pub struct HaveFile {
    pub in_port: u16,
    pub out_port: u16
}

impl Message for HaveFile {
    fn to_raw(&self) -> Vec<u8> {
        let mut og = bincode::serialize(&self).unwrap(); 

        let mut have_file = vec![0];
        have_file.append(&mut og);
        
        have_file
    }

    fn from_raw(slice: &[u8]) -> Result<Self, &'static str> {
        if !slice[0] == 0 {
            return Err("not good msg");
        }

        let have_file: Self = match bincode::deserialize(&slice[1..]) {
            Ok(v) => v,
            Err(_) => return Err("deserialising failed"),
        };
        

        Ok(have_file)
    }
}
