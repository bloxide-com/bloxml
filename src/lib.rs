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
        enum_variant::EnumVariant, message_set::MessageSet, msg_enum::MsgEnum, state::State,
    };

    Actor::new(
        "Actor",
        TEST_OUTPUT_DIR,
        vec![State::new("Create"), State::new("Update")],
        Some(MessageSet::new(
            "ActorMessage",
            vec![
                MsgEnum::new("Standard", vec![]),
                MsgEnum::new(
                    "Custom",
                    vec![
                        EnumVariant::new("CustomValue1"),
                        EnumVariant::new("CustomValue2"),
                    ],
                ),
            ],
        )),
    )
}
#[cfg(test)]
mod tests {
    use crate::blox::actor::Actor;
    use crate::create_test_actor;
    use std::fs::{self, File};
    use std::io::Write;

    const TEST_FILE: &str = "tests/test_file.xml";

    #[allow(dead_code)]
    fn serialize_actor() {
        let test_actor = create_test_actor();
        let serialized_actor =
            serde_xml_rs::to_string(&test_actor).expect("Failed to serialize actor");

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(false)
            .append(true)
            .create(false)
            .open(TEST_FILE)
            .expect("Failed to open test file");

        file.write_all(serialized_actor.as_bytes())
            .expect("Failed to write test file");
    }

    #[test]
    fn deserialize_test_file() {
        let actor: Actor =
            serde_xml_rs::from_reader(File::open(TEST_FILE).expect("Failed to open test file"))
                .expect("Failed to deserialize XML");

        let test_actor = create_test_actor();
        assert_eq!(actor, test_actor);
    }
}
