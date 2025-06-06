pub mod blox;
pub mod create;
pub mod field;
pub mod link;
pub mod method;
pub use blox::*;

pub use field::Field;
pub use link::Link;
pub use method::Method;

#[cfg(test)]
pub(crate) mod tests {
    use crate::{
        Field, Link, Method,
        actor::Actor,
        component::Component,
        enums::{EnumDef, EnumVariant},
        ext_state::{ExtState, InitArgs},
        message_handlers::{MessageHandle, MessageHandles, MessageReceiver, MessageReceivers},
        message_set::MessageSet,
        state::{State, StateEnum, States},
    };
    use pretty_assertions::assert_eq;
    use serde_json;
    use std::fs;

    const TEST_OUTPUT_DIR: &str = "tests/output";
    const TEST_FILE: &str = "tests/test_file.json";

    pub fn create_test_actor() -> Actor {
        let mut actor = Actor::new(
            "Actor",
            TEST_OUTPUT_DIR,
            create_test_states(),
            Some(create_test_message_set()),
        );
        actor.component = create_test_component();
        actor
    }

    pub fn create_test_component() -> Component {
        let message_set = Some(create_test_message_set());
        let states = create_test_states();
        let handles = create_test_handles();
        let receivers = create_test_receivers();
        let ext_state = create_test_ext_state();
        Component::new(
            "ActorComponents".to_string(),
            handles,
            receivers,
            states.clone(),
            message_set.clone(),
            ext_state,
        )
    }

    pub fn create_test_ext_state() -> ExtState {
        ExtState::new(
            "ActorExtState",
            vec![Field::new("field1", "String"), Field::new("field2", "i32")],
            vec![
                Method::new("get_custom_value", &vec![], "String", "self.custom_value"),
                Method::new("get_custom_value2", &vec![], "i32", "self.custom_value2"),
                Method::new("hello_world", &vec![], "", r#"println!("Hello, world!")"#),
            ],
            InitArgs::new("ActorInitArgs", vec![Field::new("field1", "String")]),
        )
    }

    pub fn create_test_message_set() -> MessageSet {
        MessageSet::new(EnumDef::new(
            "ActorMessageSet",
            vec![
                EnumVariant::new(
                    "CustomValue1",
                    vec![Link::new("bloxide_core::messaging::Standard")],
                ),
                EnumVariant::new("CustomValue2", vec![Link::new("CustomArgs")]),
            ],
        ))
    }

    pub fn create_test_states() -> States {
        States::new(
            vec![
                State::from("Create"),
                State::new("Update", Some("Create".to_string()), None),
            ],
            StateEnum::new(EnumDef::new("ActorStates", vec![])),
        )
    }

    pub fn create_test_handles() -> MessageHandles {
        let mut handles = MessageHandles::new("ActorHandles");
        handles.add_handle(MessageHandle::new("standard_handle", "Standard"));
        handles.add_handle(MessageHandle::new("customargs_handle", "CustomArgs"));
        handles
    }

    pub fn create_test_receivers() -> MessageReceivers {
        let mut receivers = MessageReceivers::new("ActorReceivers");
        receivers.add_receiver(MessageReceiver::new("standard_rx", "Standard"));
        receivers.add_receiver(MessageReceiver::new("customargs_rx", "CustomArgs"));
        receivers
    }

    #[expect(dead_code)]
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
