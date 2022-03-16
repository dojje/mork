use mork_message_derive::Message;
use serde::{Deserialize, Serialize};

use super::Message;

#[derive(Serialize, Deserialize, Message)]
#[message(msg_code = 4)]
pub struct YouHaveFile {
    pub code: String,
}

impl YouHaveFile {
    pub fn new(code: String) -> Self {
        Self { code }
    }
}
