use std::{error::Error, fs::OpenOptions, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    component::Component,
    ext_state::ExtState,
    message_handlers::{MessageHandle, MessageHandles, MessageReceiver, MessageReceivers},
    message_set::MessageSet,
    state::States,
};
use serde_json;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "actor")]
pub struct Actor {
    pub ident: String,
    pub path: PathBuf,
    pub component: Component,
}

impl Actor {
    pub fn new<P, S>(ident: S, path: P, states: States, message_set: Option<MessageSet>) -> Self
    where
        P: Into<PathBuf>,
        S: Into<String>,
    {
        let ident: String = ident.into();
        let (handles, receivers) = Self::create_handles(&ident, &message_set);
        let component =
            Component::new(handles, receivers, states, message_set, ExtState::default());

        Self {
            ident,
            path: path.into(),
            component,
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

    fn create_handles(
        _ident: &str,
        message_set: &Option<MessageSet>,
    ) -> (MessageHandles, MessageReceivers) {
        let mut handles = MessageHandles::new();
        let mut receivers = MessageReceivers::new();

        let Some(message_set) = message_set else {
            return (handles, receivers);
        };

        for variant in &message_set.get().variants {
            let message_type = &variant
                .args
                .first()
                .unwrap()
                .as_ref()
                .split("::")
                .last()
                .unwrap()
                .to_string();
            let handle_name = format!("{}_handle", message_type.to_lowercase());
            handles.add_handle(MessageHandle::new(handle_name, message_type));

            let receiver_name = format!("{}_rx", message_type.to_lowercase());
            receivers.add_receiver(MessageReceiver::new(receiver_name, message_type));
        }

        (handles, receivers)
    }

    pub fn message_set_ident(&self) -> String {
        self.component
            .message_set
            .as_ref()
            .map(|ms| ms.get().ident.clone())
            .unwrap_or_else(|| format!("{}_MessageSet", self.ident))
    }
}
