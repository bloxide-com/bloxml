use std::{error::Error, fs::OpenOptions, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    message_handlers::{MessageHandle, MessageHandles, MessageReceiver, MessageReceivers},
    message_set::MessageSet,
    state::{State, StateEnum, States},
};
use serde_json;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "actor")]
pub struct Actor {
    pub ident: String,
    pub path: PathBuf,
    pub states: States,
    pub message_set: Option<MessageSet>,
    #[serde(default)]
    pub message_handles: MessageHandles,
    #[serde(default)]
    pub message_receivers: MessageReceivers,
}

impl Actor {
    pub fn new<P, S>(ident: S, path: P, states: States, message_set: Option<MessageSet>) -> Self
    where
        P: Into<PathBuf>,
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            path: path.into(),
            states,
            message_set,
            message_handles: MessageHandles::new(),
            message_receivers: MessageReceivers::new(),
        }
    }

    pub fn create_mod_path(&self) -> PathBuf {
        self.path.join(self.ident.to_lowercase())
    }

    pub fn create_states_path(&self) -> PathBuf {
        self.create_mod_path().join("states")
    }

    pub fn from_json_file(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(path)?;
        serde_json::from_reader(file).map_err(From::from)
    }

    pub fn with_states<P, S>(
        ident: S,
        path: P,
        states: Vec<State>,
        state_enum: StateEnum,
        message_set: Option<MessageSet>,
    ) -> Self
    where
        P: Into<PathBuf>,
        S: Into<String>,
    {
        Self::new(ident, path, States::new(states, state_enum), message_set)
    }

    /// Add a default set of message handles and receivers for this actor
    pub fn with_default_messages(mut self) -> Self {
        // Get the message set name
        let message_set_name = self
            .message_set
            .as_ref()
            .map(|ms| ms.get().ident.clone())
            .unwrap_or_else(|| format!("{}MessageSet", self.ident));

        let actor_name = self.ident.clone();

        // Add standard message handle for core messaging
        let standard_handle = MessageHandle::standard("standard_handle");
        self.message_handles.add_handle(standard_handle);

        // Add actor-specific message handle
        let actor_handle = MessageHandle::new(
            format!("{}_handle", actor_name.to_lowercase()),
            message_set_name.clone(),
        );
        self.message_handles.add_handle(actor_handle);

        // Add standard message receiver
        let standard_rx = MessageReceiver::standard("std_rx");
        self.message_receivers.add_receiver(standard_rx);

        // Add actor-specific message receiver
        let actor_rx = MessageReceiver::new(
            format!("{}_rx", actor_name.to_lowercase()),
            message_set_name.clone(),
        );
        self.message_receivers.add_receiver(actor_rx);

        // Add handles and receivers for each message in the message set
        if let Some(message_set) = &self.message_set {
            for variant in &message_set.get().variants {
                // Generate a handle for each message variant
                let message_type = &variant.ident;
                let handle_name = format!("{}_handle", message_type.to_lowercase());

                // Skip if we already have a similar handle
                if !self
                    .message_handles
                    .handles
                    .iter()
                    .any(|h| h.name == handle_name)
                {
                    let handle = MessageHandle::new(handle_name, message_type.to_string());
                    self.message_handles.add_handle(handle);
                }

                // Generate a receiver for each message variant
                let receiver_name = format!("{}_rx", message_type.to_lowercase());

                // Skip if we already have a similar receiver
                if !self
                    .message_receivers
                    .receivers
                    .iter()
                    .any(|r| r.name == receiver_name)
                {
                    let receiver = MessageReceiver::new(receiver_name, message_type.to_string());
                    self.message_receivers.add_receiver(receiver);
                }
            }
        }

        self
    }
}
