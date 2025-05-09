use serde::{Deserialize, Serialize};

use super::enum_variant::EnumVariant;

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "msgenum")]
pub struct MsgEnum {
    #[serde(rename = "@ident")]
    pub ident: String,
    #[serde(rename = "enumvariant", default)]
    pub variants: Vec<EnumVariant>,
}

impl MsgEnum {
    pub fn new<S>(ident: S, variants: Vec<EnumVariant>) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            variants,
        }
    }
}
