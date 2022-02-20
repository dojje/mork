use message_derive::Message;
use serde::{Serialize, Deserialize};

use super::Message;

#[derive(Serialize, Deserialize, Clone, Message)]
#[message(msg_code = 1)]
pub struct IHaveCode {
    pub code: String,
}

impl IHaveCode {
    pub fn new(code: String) -> Self {
        Self {
            code
        }
    }
}
