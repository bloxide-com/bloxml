use serde::{Deserialize, Serialize};

use super::enums::{EnumDef, EnumVariant};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "state_enum")]
pub struct StateEnum(pub EnumDef);

impl StateEnum {
    pub fn new(enum_def: EnumDef) -> Self {
        Self(enum_def)
    }

    pub fn get(&self) -> &EnumDef {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename = "state")]
pub struct State {
    pub ident: String,
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<EnumVariant>>,
}

impl State {
    pub fn new<S>(ident: S, parent: Option<String>, variants: Option<Vec<EnumVariant>>) -> Self
    where
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            parent,
            variants,
        }
    }
}

impl From<&str> for State {
    fn from(ident: &str) -> Self {
        Self::new(ident, None, None)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct States {
    pub state_enum: StateEnum,
    pub states: Vec<State>,
}

impl States {
    pub fn new(states: Vec<State>, state_enum: StateEnum) -> Self {
        Self { state_enum, states }
    }

    pub fn get_state(&self, name: &str) -> Option<&State> {
        self.states.iter().find(|s| s.ident == name)
    }

    pub fn validate(&self) -> Result<(), String> {
        for state in &self.states {
            if let Some(parent) = &state.parent {
                if !self.states.iter().any(|s| &s.ident == parent) {
                    return Err(format!(
                        "State '{}' has unknown parent '{}'",
                        state.ident, parent
                    ));
                }
            }
        }

        for variant in &self.state_enum.get().variants {
            for arg in &variant.args {
                // Try to check if the argument references a state
                let arg_str = arg.to_string();
                if !arg_str.contains("::") && !self.states.iter().any(|s| s.ident == arg_str) {
                    return Err(format!(
                        "Variant '{}' references unknown state '{}'",
                        variant.ident, arg_str
                    ));
                }
            }
        }

        Ok(())
    }
}
