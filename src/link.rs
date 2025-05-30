use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
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

impl Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Link {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Link {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Link {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
