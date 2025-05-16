use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "enumvariant")]
pub struct EnumVariant {
    pub ident: String,
    pub args: Vec<Link>,
}

impl EnumVariant {
    pub fn new<S>(ident: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            args: vec![],
        }
    }
}

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "link")]
pub struct Link(String);

impl Link {
    pub fn new<S>(link: S) -> Self
    where
        S: Into<String>,
    {
        Self(link.into())
    }
}

impl AsRef<str> for Link {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
