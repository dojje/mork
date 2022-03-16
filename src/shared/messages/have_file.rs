use mork_message_derive::Message;
use serde::{Deserialize, Serialize};

use super::Message;

#[derive(Deserialize, Serialize, Message)]
#[message(msg_code = 0)]
pub struct HaveFile {
    pub file_name: String,
    pub file_len: u64,
}

impl HaveFile {
    pub fn new(file_name: String, file_len: u64) -> Self {
        Self {
            file_name,
            file_len,
        }
    }
}
