use serde::{Deserialize, Serialize};

use crate::{Method, create::ToRust, field::Field};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Default)]
pub struct ExtState {
    ident: String,
    #[serde(default)]
    fields: Vec<Field>,
    #[serde(default)]
    methods: Vec<Method>,
}

impl ExtState {
    pub fn new<S>(ident: S, fields: Vec<Field>, methods: Vec<Method>) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            fields,
            methods,
        }
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

    pub fn methods(&self) -> &[Method] {
        &self.methods
    }
}

impl ToRust for ExtState {
    fn to_rust(&self) -> String {
        let fields = self
            .fields
            .iter()
            .map(ToRust::to_rust)
            .collect::<Vec<_>>()
            .join(",\n\t");

        let params = self
            .fields
            .iter()
            .map(|f| format!("{}: {}", f.ident(), f.ty()))
            .collect::<Vec<_>>()
            .join(", ");

        let fields_init = self
            .fields
            .iter()
            .map(|f| f.ident())
            .collect::<Vec<_>>()
            .join(",\n\t");

        let methods = self
            .methods
            .iter()
            .map(ToRust::to_rust)
            .collect::<Vec<_>>()
            .join("\n\t");

        format!(
            r#"pub struct {ident} {{
    {fields}
}}

impl {ident} {{
    pub fn new({params}) -> Self {{
        Self {{
            {fields_init}
        }}
    }}

    {methods}
}}"#,
            ident = self.ident,
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;

    const EXT_STATE_JSON: &str = "tests/ext_state.json";

    pub(crate) fn create_ext_state() -> ExtState {
        ExtState::new(
            "ActorExtState",
            vec![Field::new("field1", "String"), Field::new("field2", "i32")],
            vec![
                Method::new("get_custom_value", &vec![], "String", "self.custom_value"),
                Method::new("get_custom_value2", &vec![], "i32", "self.custom_value2"),
                Method::new("hello_world", &vec![], "", r#"println!("Hello, world!")"#),
            ],
        )
    }

    #[expect(dead_code)]
    fn serialize_ext_state() {
        let ext_state = create_ext_state();
        let json_str = serde_json::to_string_pretty(&ext_state).unwrap();
        fs::write(EXT_STATE_JSON, json_str).unwrap();
    }

    #[test]
    fn test_deserialize_from_json() {
        let json_str = fs::read_to_string(EXT_STATE_JSON).unwrap();
        let ext_state: ExtState = serde_json::from_str(&json_str).unwrap();

        assert_eq!(ext_state.ident(), "ActorExtState");
        assert_eq!(ext_state.fields().len(), 2);

        let fields = ext_state.fields();
        let expected_fields = vec![Field::new("field1", "String"), Field::new("field2", "i32")];

        assert_eq!(fields, &expected_fields);
    }
}
