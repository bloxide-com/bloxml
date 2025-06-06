use serde::{Deserialize, Serialize};

use crate::create::ToRust;

/// Defines a message handle for sending messages
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct MessageHandle {
    /// Name of the handle
    pub ident: String,
    /// Type of message this handle sends
    pub message_type: String,
}

impl MessageHandle {
    /// Create a new message handle
    pub fn new(ident: impl Into<String>, message_type: impl Into<String>) -> Self {
        Self {
            ident: ident.into(),
            message_type: message_type.into(),
        }
    }

    /// Create a standard system message handle
    pub fn standard(name: impl Into<String>) -> Self {
        Self::new(name, "StandardMessage")
    }
}

impl ToRust for MessageHandle {
    fn to_rust(&self) -> String {
        format!(
            "pub {}: <TokioRuntime as Runtime>::MessageHandle<{}>",
            self.ident, self.message_type
        )
    }
}

/// Defines a message receiver for receiving messages
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct MessageReceiver {
    /// Name of the receiver
    pub ident: String,
    /// Type of message this receiver accepts
    pub message_type: String,
}

impl MessageReceiver {
    /// Create a new message receiver
    pub fn new(ident: impl Into<String>, message_type: impl Into<String>) -> Self {
        Self {
            ident: ident.into(),
            message_type: message_type.into(),
        }
    }

    /// Create a standard system message receiver
    pub fn standard(ident: impl Into<String>) -> Self {
        Self::new(ident, "StandardMessage")
    }
}

impl ToRust for MessageReceiver {
    fn to_rust(&self) -> String {
        format!(
            "pub {}: <TokioRuntime as Runtime>::MessageHandle<{}> as MessageSender>::ReceiverType",
            self.ident, self.message_type
        )
    }
}

/// Collection of message handles for an actor
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Default, Clone)]
pub struct MessageHandles {
    /// Name of the struct
    pub ident: String,
    /// All handles for this actor
    pub handles: Vec<MessageHandle>,
}

impl MessageHandles {
    /// Create a new empty collection of message handles
    pub fn new<S: Into<String>>(ident: S) -> Self {
        Self {
            ident: ident.into(),
            handles: Vec::new(),
        }
    }

    /// Add a handle to the collection
    pub fn add_handle(&mut self, handle: MessageHandle) {
        self.handles.push(handle);
    }

    /// Get a handle by name
    pub fn get_handle(&self, name: &str) -> Option<&MessageHandle> {
        self.handles.iter().find(|h| h.ident == name)
    }
}

impl ToRust for MessageHandles {
    fn to_rust(&self) -> String {
        let fields = self
            .handles
            .iter()
            .map(ToRust::to_rust)
            .collect::<Vec<_>>()
            .join(",\n\t");
        format!(
            "pub struct {ident} {{
    {fields}
}}",
            ident = self.ident
        )
    }
}

/// Collection of message receivers for an actor
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Default, Clone)]
pub struct MessageReceivers {
    /// Name of the receivers struct
    pub ident: String,
    /// All receivers for this actor
    pub receivers: Vec<MessageReceiver>,
}

impl MessageReceivers {
    /// Create a new empty collection of message receivers
    pub fn new<S: Into<String>>(ident: S) -> Self {
        Self {
            ident: ident.into(),
            receivers: Vec::new(),
        }
    }

    /// Add a receiver to the collection
    pub fn add_receiver(&mut self, receiver: MessageReceiver) {
        self.receivers.push(receiver);
    }

    /// Get a receiver by name
    pub fn get_receiver(&self, name: &str) -> Option<&MessageReceiver> {
        self.receivers.iter().find(|r| r.ident == name)
    }
}

impl ToRust for MessageReceivers {
    fn to_rust(&self) -> String {
        let fields = self
            .receivers
            .iter()
            .map(ToRust::to_rust)
            .collect::<Vec<_>>()
            .join(",\n\t");
        format!(
            "pub struct {ident} {{
    {fields}
}}",
            ident = self.ident
        )
    }
}
