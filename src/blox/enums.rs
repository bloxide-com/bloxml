use crate::Link;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "enum")]
pub struct EnumDef {
    pub ident: String,
    #[serde(rename = "enumvariant", default)]
    pub variants: Vec<EnumVariant>,
}

impl EnumDef {
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

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "enumvariant")]
pub struct EnumVariant {
    pub ident: String,
    pub args: Vec<Link>,
}

impl EnumVariant {
    pub fn new<S>(ident: S, args: Vec<Link>) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            args,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Link;

    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json;
    use std::fs;
    const TEST_ENUM_FILE: &str = "tests/enum_file.json";
    const EXPECTED_ENUM: &str = "tests/expected_enum.json";

    fn create_expected_enum() -> EnumDef {
        EnumDef::new(
            "Custom",
            vec![
                EnumVariant::new(
                    "CustomValue1",
                    vec![Link::new("bloxide_core::messaging::Standard")],
                ),
                EnumVariant::new("CustomValue2", vec![]),
            ],
        )
    }

    #[test]
    fn msg_enum_sanity_test() {
        let expected = create_expected_enum();
        let serialized = serde_json::to_string(&expected).expect("Failed to serialize MsgEnum");
        let deserialized =
            serde_json::from_str(&serialized).expect("Failed to deserialize MsgEnum");
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_serialize_enum() {
        let expected = create_expected_enum();
        let serialized =
            serde_json::to_string_pretty(&expected).expect("Failed to serialize MsgEnum");
        fs::write(EXPECTED_ENUM, serialized).expect("Failed to write expected enum");
    }

    #[test]
    fn test_deserialize_enum_json() {
        // Create the expected msg_enum with the expected structure
        let expected = create_expected_enum();

        // Deserialize the JSON file
        let json_content =
            fs::read_to_string(TEST_ENUM_FILE).expect("Failed to read enum test file");
        let deserialized: EnumDef =
            serde_json::from_str(&json_content).expect("Failed to deserialize MsgEnum JSON");

        assert_eq!(deserialized, expected,);
    }

    #[test]
    fn test_serialize_enum_json() {
        let expected = create_expected_enum();

        let serialized = serde_json::to_string(&expected).expect("Failed to serialize MsgEnum");
        let deserialized: EnumDef =
            serde_json::from_str(&serialized).expect("Failed to deserialize MsgEnum");

        assert_eq!(deserialized, expected);
    }
}
