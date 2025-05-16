use serde::{Deserialize, Serialize};

use super::enums::EnumDef;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct MessageSet(EnumDef);

impl MessageSet {
    pub fn new(enum_def: EnumDef) -> Self {
        Self(enum_def)
    }

    pub fn get(&self) -> &EnumDef {
        &self.0
    }
}
