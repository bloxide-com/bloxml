use serde::{Deserialize, Serialize};

use super::enums::EnumDef;

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "stateenum")]
pub struct StateEnum(EnumDef);

impl StateEnum {
    pub fn new(enum_def: EnumDef) -> Self {
        Self(enum_def)
    }

    pub fn get(&self) -> &EnumDef {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "state")]
pub struct State {
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
