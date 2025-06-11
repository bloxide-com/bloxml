use serde::{Deserialize, Serialize};

use super::enums::EnumDef;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct MessageSet {
    pub def: EnumDef,
    #[serde(default)]
    pub custom_types: Vec<EnumDef>,
}

impl MessageSet {
    pub fn new(def: EnumDef) -> Self {
        Self {
            def,
            custom_types: Vec::new(),
        }
    }

    pub fn with_custom_types(def: EnumDef, custom_types: Vec<EnumDef>) -> Self {
        Self { def, custom_types }
    }

    pub fn get(&self) -> &EnumDef {
        &self.def
    }
}
