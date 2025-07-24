use serde::{Deserialize, Serialize};

use super::enums::{EnumDef, EnumVariant};
use crate::create::{ActorGenerator, ToRust};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
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
        if let Some(state) = self.states.iter().find(|state| {
            // find state with a parent not in the list of states
            state
                .parent
                .as_ref()
                .is_some_and(|parent| !self.states.iter().any(|s| &s.ident == parent))
        }) {
            return Err(format!(
                "State '{}' has unknown parent '{}'",
                state.ident,
                state.parent.as_ref().unwrap()
            ));
        }

        for variant in &self.state_enum.get().variants {
            variant
                .args
                .iter()
                .find_map(|arg| {
                    // check for variant args that are not states
                    let arg_str = arg.to_string();
                    if !arg_str.contains("::") && !self.states.iter().any(|s| s.ident == arg_str) {
                        Some(format!(
                            "Variant '{ident}' references unknown state '{arg_str}'",
                            ident = variant.ident
                        ))
                    } else {
                        None
                    }
                })
                .map_or(Ok(()), Err)?;
        }
        Ok(())
    }
}

impl ToRust for State {
    fn to_rust(&self, generator: &ActorGenerator) -> String {
        let state_name = &self.ident;
        let component_type = generator.component_type();
        let message_set = generator.message_set();

        format!(
            r#"/// State implementation for {state_name} state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct {state_name};

impl State<{component_type}> for {state_name} {{
    fn handle_message(
        &self,
        _state_machine: &mut StateMachine<{component_type}>,
        _message: {message_set},
    ) -> Option<Transition<<{component_type} as Components>::States, {message_set}>> {{
        None
    }}
}}"#
        )
    }
}

impl ToRust for StateEnum {
    fn to_rust(&self, generator: &ActorGenerator) -> String {
        let enum_def = self.get();
        let enum_name = &enum_def.ident;
        let component_type = generator.component_type();
        let message_set = generator.message_set();

        // Use actual states from the generator, not the empty enum_def.variants
        let actual_states = &generator.actor().component.states.states;

        let variants = actual_states
            .iter()
            .map(|state| {
                format!(
                    "    /// {} state\n    {}({}),",
                    state.ident, state.ident, state.ident
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let handle_message_arms = actual_states.iter()
            .map(|state| format!("            {enum_name}::{state_name}(state) => state.handle_message(state_machine, message),", state_name = state.ident))
            .collect::<Vec<_>>()
            .join("\n");

        let on_entry_arms = actual_states.iter()
            .map(|state| format!("            {enum_name}::{state_name}(state) => state.on_entry(state_machine),", state_name = state.ident))
            .collect::<Vec<_>>()
            .join("\n");

        let on_exit_arms = actual_states
            .iter()
            .map(|state| {
                format!(
                    "            {enum_name}::{state_name}(state) => state.on_exit(state_machine),",
                    state_name = state.ident
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let parent_arms = actual_states
            .iter()
            .map(|state| {
                format!(
                    "            {enum_name}::{state_name}(state) => state.parent(),",
                    state_name = state.ident
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"/// Enumeration of all possible states for the actor's state machine
#[derive(Clone, PartialEq, Debug)]
pub enum {enum_name} {{
{variants}
}}

impl State<{component_type}> for {enum_name} {{
    /// Handles incoming messages and returns a transition to a new state if needed
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_type}>,
        message: {message_set},
    ) -> Option<Transition<<{component_type} as Components>::States, {message_set}>> {{
        match self {{
{handle_message_arms}
        }}
    }}

    /// Executes actions when entering a state
    fn on_entry(&self, state_machine: &mut StateMachine<{component_type}>) {{
        match self {{
{on_entry_arms}
        }}
    }}

    /// Executes actions when exiting a state
    fn on_exit(&self, state_machine: &mut StateMachine<{component_type}>) {{
        match self {{
{on_exit_arms}
        }}
    }}

    /// Returns the parent state in the state machine hierarchy
    fn parent(&self) -> {enum_name} {{
        match self {{
{parent_arms}
        }}
    }}
}}

impl StateEnum for {enum_name} {{
    fn new() -> Self {{
        Self::default()
    }}
}}

impl Default for {enum_name} {{
    fn default() -> Self {{
        {enum_name}::Uninit(Uninit)
    }}
}}"#
        )
    }
}

impl ToRust for States {
    fn to_rust(&self, generator: &ActorGenerator) -> String {
        let state_impls = self
            .states
            .iter()
            .map(|state| state.to_rust(generator))
            .collect::<Vec<_>>()
            .join("\n\n");

        let state_enum_impl = self.state_enum.to_rust(generator);

        format!("{state_impls}\n\n{state_enum_impl}")
    }
}
