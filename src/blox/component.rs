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
    pub fn new(
        message_handles: MessageHandles,
        message_receivers: MessageReceivers,
        states: States,
        message_set: Option<MessageSet>,
        ext_state: ExtState,
    ) -> Self {
        Self {
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
        let handles = self.message_handles.to_rust();
        let receivers = self.message_receivers.to_rust();

        format!(
            r#"/// Message handles for sending messages
{handles}

/// Message receivers for receiving messages
{receivers}"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{blox::message_handlers::{MessageHandle, MessageReceiver}, tests::create_test_states};

    #[test]
    fn test_to_rust() {
        let mut handles = MessageHandles::new();
        handles.add_handle(MessageHandle::new("test_handle", "TestMessage"));

        let mut receivers = MessageReceivers::new();
        receivers.add_receiver(MessageReceiver::new("test_rx", "TestMessage"));

        let component = Component::new(
            handles,
            receivers,
            create_test_states(),
            None,
            ExtState::default(),
        );
        let rust_code = component.to_rust();

        assert!(rust_code.contains("pub struct MessageHandles"));
        assert!(rust_code.contains("pub struct MessageReceivers"));
        assert!(rust_code.contains("pub test_handle: TokioMessageHandle<TestMessage>"));
        assert!(rust_code.contains("pub test_rx: Receiver<TestMessage>"));
    }
}
