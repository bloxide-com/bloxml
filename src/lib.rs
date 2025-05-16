pub mod blox;
pub mod create;

pub use blox::*;

#[cfg(test)]
use blox::actor::Actor;
#[cfg(test)]
const TEST_OUTPUT_DIR: &str = "tests/output";

#[cfg(test)]
pub fn create_test_actor() -> Actor {
    use blox::{
        enum_variant::{EnumVariant, Link},
        enums::EnumDef,
        message_set::MessageSet,
        state::State,
    };

    Actor::new(
        "Actor",
        TEST_OUTPUT_DIR,
        vec![State::new("Create"), State::new("Update")],
        Some(MessageSet::new(EnumDef::new(
            "ActorMessage",
            vec![
                EnumVariant {
                    ident: "CustomValue1".to_string(),
                    args: vec![Link::new("bloxide_core::messaging::Standard")],
                },
                EnumVariant::new("CustomValue2"),
            ],
        ))),
    )
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json;

    use crate::blox::actor::Actor;
    use crate::create_test_actor;
    use std::fs;

    const TEST_FILE: &str = "tests/test_file.json";

    #[test]
    #[ignore]
    fn serialize_actor() {
        let test_actor = create_test_actor();
        let serialized_actor =
            serde_json::to_string_pretty(&test_actor).expect("Failed to serialize actor");
        fs::write(TEST_FILE, serialized_actor).expect("Failed to write test file");
    }

    #[test]
    fn deserialize_test_file() {
        let contents = fs::read_to_string(TEST_FILE).expect("Failed to read test file");
        let actor: Actor = serde_json::from_str(&contents).expect("Failed to deserialize JSON");

        let test_actor = create_test_actor();
        assert_eq!(actor, test_actor);
    }

    #[test]
    fn sanity_test() {
        let expected = create_test_actor();

        let serialized_actor = serde_json::to_string(&expected).expect("Failed to serialize actor");
        let deserialized_actor: Actor =
            serde_json::from_str(&serialized_actor).expect("Failed to deserialize actor");

        assert_eq!(expected, deserialized_actor);
    }
}
