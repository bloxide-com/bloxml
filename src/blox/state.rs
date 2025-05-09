use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "state")]
pub struct State {
    #[serde(rename = "@ident")]
    pub ident: String,
    pub parent: Option<Box<State>>,
}

impl State {
    pub fn new<S>(ident: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            parent: None,
        }
    }
}
