use serde::{Deserialize, Serialize};

/// Defines a message handle for sending messages
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct MessageHandle {
    /// Name of the handle
    pub name: String,
    /// Type of message this handle sends
    pub message_type: String,
    /// Additional metadata
    pub metadata: Vec<String>,
}

impl MessageHandle {
    /// Create a new message handle
    pub fn new(name: impl Into<String>, message_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message_type: message_type.into(),
            metadata: Vec::new(),
        }
    }

    /// Create a standard system message handle
    pub fn standard(name: impl Into<String>) -> Self {
        Self::new(name, "StandardMessage")
    }
}

/// Defines a message receiver for receiving messages
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct MessageReceiver {
    /// Name of the receiver
    pub name: String,
    /// Type of message this receiver accepts
    pub message_type: String,
    /// Additional metadata
    pub metadata: Vec<String>,
}

impl MessageReceiver {
    /// Create a new message receiver
    pub fn new(name: impl Into<String>, message_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message_type: message_type.into(),
            metadata: Vec::new(),
        }
    }

    /// Create a standard system message receiver
    pub fn standard(name: impl Into<String>) -> Self {
        Self::new(name, "StandardMessage")
    }
}

/// Collection of message handles for an actor
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Default, Clone)]
pub struct MessageHandles {
    /// All handles for this actor
    pub handles: Vec<MessageHandle>,
}

impl MessageHandles {
    /// Create a new empty collection of message handles
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    /// Add a handle to the collection
    pub fn add_handle(&mut self, handle: MessageHandle) {
        self.handles.push(handle);
    }

    /// Get a handle by name
    pub fn get_handle(&self, name: &str) -> Option<&MessageHandle> {
        self.handles.iter().find(|h| h.name == name)
    }
}

/// Collection of message receivers for an actor
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Default, Clone)]
pub struct MessageReceivers {
    /// All receivers for this actor
    pub receivers: Vec<MessageReceiver>,
}

impl MessageReceivers {
    /// Create a new empty collection of message receivers
    pub fn new() -> Self {
        Self {
            receivers: Vec::new(),
        }
    }

    /// Add a receiver to the collection
    pub fn add_receiver(&mut self, receiver: MessageReceiver) {
        self.receivers.push(receiver);
    }

    /// Get a receiver by name
    pub fn get_receiver(&self, name: &str) -> Option<&MessageReceiver> {
        self.receivers.iter().find(|r| r.name == name)
    }
}
