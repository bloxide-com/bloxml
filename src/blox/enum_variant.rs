use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(transparent)]
pub struct EnumVariant {
    pub ident: String,
}

impl EnumVariant {
    pub fn new<S>(ident: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
        }
    }
}
