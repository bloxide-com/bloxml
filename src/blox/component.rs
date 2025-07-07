use super::{
    ext_state::ExtState,
    message_handlers::{MessageHandles, MessageReceivers},
    message_set::MessageSet,
    state::States,
};
use crate::create::ToRust;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Component {
    pub ident: String,
    pub states: States,
    pub message_set: Option<MessageSet>,
    #[serde(default)]
    pub message_handles: MessageHandles,
    #[serde(default)]
    pub message_receivers: MessageReceivers,
    #[serde(default)]
    pub ext_state: ExtState,
}

impl Component {
    pub fn new<S: Into<String>>(
        ident: S,
        message_handles: MessageHandles,
        message_receivers: MessageReceivers,
        states: States,
        message_set: Option<MessageSet>,
        ext_state: ExtState,
    ) -> Self {
        Self {
            ident: ident.into(),
            message_handles,
            message_receivers,
            states,
            message_set,
            ext_state,
        }
    }
}

impl ToRust for Component {
    fn to_rust(&self) -> String {
        let actor_name = &self.ident.split("Components").next().unwrap();
        let component_name = &self.ident;
        let ext_state_name = &self.ext_state.ident();
        let states_name = &self.states.state_enum.get().ident;
        let message_set_name = self
            .message_set
            .as_ref()
            .map(|ms| ms.get().ident.clone())
            .unwrap_or_else(|| format!("{actor_name}MessageSet"));

        let handles_ident = &self.message_handles.ident;
        let receivers_ident = &self.message_receivers.ident;

        let handles = self.message_handles.to_rust();
        let receivers = self.message_receivers.to_rust();

        format!(
            r#"//! # {actor_name} Components
//!
//! This module defines the component structure for the {actor_name} Blox.
//! It specifies the states, message types, extended state, and communication
//! channels that make up the {actor_name} component.

use super::{{
    ext_state::{ext_state_name},
    messaging::{message_set_name},
    states::{states_name},
}};
use bloxide_tokio::{{
    components::{{Components, Runtime}},
    TokioMessageHandle,
    messaging::{{Message, MessageSet, MessageSender, StandardPayload}},
    TokioRuntime,
    }};

/// Defines the structure of the {actor_name} Blox component
pub struct {component_name};

impl Components for {component_name} {{
    type States = {states_name};
    type MessageSet = {message_set_name};
    type ExtendedState = {ext_state_name};
    type Receivers = {receivers_ident};
    type Handles = {handles_ident};
}}

/// Receiver channels for the {actor_name} component
{receivers}

/// Message handles for sending messages from the {actor_name} component
{handles}
"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blox::message_handlers::{MessageHandle, MessageReceiver},
        tests::create_test_states,
    };

    #[test]
    fn test_to_rust() {
        let mut handles = MessageHandles::new("ActorHandles");
        handles.add_handle(MessageHandle::new("test_handle", "TestMessage"));

        let mut receivers = MessageReceivers::new("ActorReceivers");
        receivers.add_receiver(MessageReceiver::new("test_rx", "TestMessage"));

        let component = Component::new(
            "ActorComponents".to_string(),
            handles,
            receivers,
            create_test_states(),
            None,
            ExtState::default(),
        );
        let rust_code = component.to_rust();

        assert!(rust_code.contains("pub struct ActorHandles"));
        assert!(rust_code.contains("pub struct ActorReceivers"));
        assert!(
            rust_code
                .contains("pub test_handle: TokioMessageHandle<TestMessage>")
        );
        assert!(rust_code.contains("pub test_rx: <<bloxide_tokio::TokioRuntime as Runtime>::MessageHandle<TestMessage> as MessageSender>::ReceiverType"));
    }
}
