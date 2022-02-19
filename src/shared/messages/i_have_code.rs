use message_derive::Message;
use serde::{Serialize, Deserialize};

use super::Message;

#[derive(Serialize, Deserialize, Clone, Message)]
#[message(msg_code = 2)]
pub struct IHaveCode {
    pub code: String,
}
