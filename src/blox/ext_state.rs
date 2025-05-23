use serde::{Deserialize, Serialize};

use crate::field::Field;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct ExtState {
    ident: String,
    fields: Vec<Field>,
}

impl ExtState {
    pub fn new(ident: String, fields: Vec<Field>) -> Self {
        Self { ident, fields }
    }

    pub fn ident(&self) -> &str {
        &self.ident
    }

    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}
