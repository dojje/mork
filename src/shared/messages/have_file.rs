use message_derive::Message;
use serde::{Deserialize, Serialize};


use super::Message;

#[derive(Deserialize, Serialize, Message)]
#[message(msg_code = 0)]
pub struct HaveFile {
    pub file_name: String
}

impl HaveFile {
    pub fn new(file_name: String) -> Self {
        Self {
            file_name
        }
    }
}
