use std::{error::Error, fs::OpenOptions, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{
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
}
