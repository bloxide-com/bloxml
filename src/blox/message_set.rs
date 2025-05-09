use serde::{Deserialize, Serialize};

use super::msg_enum::MsgEnum;

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "messageset")]
pub struct MessageSet {
    #[serde(rename = "@ident")]
    pub ident: String,
    #[serde(rename = "msgenum")]
    pub enums: Vec<MsgEnum>,
}

impl MessageSet {
    pub fn new<S>(ident: S, enums: Vec<MsgEnum>) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            enums,
        }
    }
}
