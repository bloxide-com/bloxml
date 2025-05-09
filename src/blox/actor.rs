use std::{error::Error, fs::OpenOptions, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::{message_set::MessageSet, state::State};

#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "actor")]
pub struct Actor {
    #[serde(rename = "@ident")]
    pub ident: String,
    pub path: PathBuf,
    #[serde(default)]
    pub states: Vec<State>,
    #[serde(rename = "messageset")]
    pub message_set: Option<MessageSet>,
}

impl Actor {
    pub fn new<P, S>(ident: S, path: P, states: Vec<State>, message_set: Option<MessageSet>) -> Self
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

    pub fn from_xml_file(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(path)?;
        serde_xml_rs::from_reader(file).map_err(From::from)
    }
}
