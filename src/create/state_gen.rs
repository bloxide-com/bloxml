use crate::blox::state::{State, States};
use std::error::Error;

/// Generate a state implementation for a specific State in the States collection
pub fn generate_inner_states(state: &State) -> Result<String, Box<dyn Error>> {
    let parent_state = state.parent.clone().unwrap_or_else(|| "NONE".to_string());

    let state_name = &state.ident;

    let impl_content = format!(
        r#"use bloxide_core::{{components::Components, message::MessageSet, state_machine::{{StateMachine, State, Transition}}}};
use log::trace;

/// State implementation for {state_name} state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct {state_name};

impl State<Components> for {state_name} {{
    fn on_entry(&self, state_machine: &mut StateMachine<Components>) {{
        trace!("State on_entry: {state_name}");
    }}

    fn on_exit(&self, state_machine: &mut StateMachine<Components>) {{
        trace!("State on_exit: {state_name}");
    }}

    fn parent(&self) -> Components::States {{
        {parent_relation}
    }}

    fn handle_message(
        &self,
        state_machine: &mut StateMachine<Components>,
        message: Components::MessageSet,
    ) -> Option<Transition<Components::States, Components::MessageSet>> {{
        None
    }}
}}
"#,
        state_name = state_name,
        parent_relation = if parent_state == "NONE" {
            "Components::States::from(self.clone()) // No parent state".to_string()
        } else {
            format!("Components::States::from({}::new())", parent_state)
        }
    );
    Ok(impl_content)
}

/// Generate a unified StateEnum implementation that contains all states
pub fn generate_state_enum_impl(states: &States) -> Result<String, Box<dyn Error>> {
    let enum_name = states.state_enum.get().ident.clone();

    // Generate module imports for each State
    let imports = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{}mod {};\nuse {}::{};\n",
            acc,
            state.ident.to_lowercase(),
            state.ident.to_lowercase(),
            state.ident
        )
    });

    // Generate enum variants
    let variants = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{}    /// {state_name} state\n    {state_name}({state_name}),\n",
            acc,
            state_name = state.ident
        )
    });

    // Generate match arms for handle_message method
    let handle_message_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!("{}            {enum_name}::{state_name}(state) => state.handle_message(state_machine, message),\n", 
            acc,
            enum_name = enum_name,
            state_name = state.ident
        )
    });

    // Generate match arms for on_entry method
    let on_entry_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{}            {enum_name}::{state_name}(state) => state.on_entry(state_machine),\n",
            acc,
            enum_name = enum_name,
            state_name = state.ident
        )
    });

    // Generate match arms for on_exit method
    let on_exit_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{}            {enum_name}::{state_name}(state) => state.on_exit(state_machine),\n",
            acc,
            enum_name = enum_name,
            state_name = state.ident
        )
    });

    // Generate match arms for parent method
    let parent_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{}            {enum_name}::{state_name}(state) => state.parent(),\n",
            acc,
            enum_name = enum_name,
            state_name = state.ident
        )
    });

    let impl_content = format!(
        r#"use bloxide_core::{{components::Components, message::MessageSet, state_machine::{{StateMachine, State, Transition}}}};
use log::trace;

{imports}

/// Enumeration of all possible states for the actor's state machine
#[derive(Clone, PartialEq, Debug)]
pub enum {enum_name} {{
{variants}}}

impl State<Components> for {enum_name} {{
    /// Handles incoming messages and returns a transition to a new state if needed
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<Components>,
        message: Components::MessageSet,
    ) -> Option<Transition<Components::States, Components::MessageSet>> {{
        match self {{
{handle_message_arms}
        }}
    }}

    /// Executes actions when entering a state
    fn on_entry(&self, state_machine: &mut StateMachine<Components>) {{
        match self {{
{on_entry_arms}
        }}
    }}

    /// Executes actions when exiting a state
    fn on_exit(&self, state_machine: &mut StateMachine<Components>) {{
        match self {{
{on_exit_arms}
        }}
    }}

    /// Returns the parent state in the state machine hierarchy
    fn parent(&self) -> Components::States {{
        match self {{
{parent_arms}
        }}
    }}
}}
"#,
        enum_name = enum_name,
        imports = imports,
        variants = variants,
        handle_message_arms = handle_message_arms,
        on_entry_arms = on_entry_arms,
        on_exit_arms = on_exit_arms,
        parent_arms = parent_arms,
    );

    Ok(impl_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blox::{
        enums::EnumDef,
        state::{State, StateEnum, States},
    };

    #[test]
    fn test_generate_state_impls() {
        // Create a simple state
        let state = State::from("Create");

        // Generate implementation
        let impl_content = generate_inner_states(&state).expect("Failed to generate state impls");
        let ident = state.ident;
        eprintln!("State impl for {ident}: {impl_content}");

        // Verify file contents
        assert!(impl_content.contains(&format!("pub struct {ident}")));
        assert!(impl_content.contains(&format!("impl State<Components> for {ident}")));
    }

    #[test]
    fn test_generate_state_enum_impl() {
        // Create explicit state enum
        let state_enum = StateEnum::new(EnumDef::new("ActorStates", vec![]));

        // Create states
        let states = States::new(
            vec![
                State::from("Create"),
                State::new("Update", Some("Create".to_string()), None),
                State::new("Delete", Some("Update".to_string()), None),
            ],
            state_enum,
        );

        // Generate implementation for the entire state enum
        let impl_content =
            generate_state_enum_impl(&states).expect("Failed to generate state enum impl");
        eprintln!("State enum impl: {}", impl_content);

        // Verify basic structure
        assert!(impl_content.contains("pub enum ActorStates"));
        assert!(impl_content.contains("impl State<Components> for ActorStates"));
        assert!(!impl_content.contains("Initial")); // No Initial or Default states

        // Verify all states are included as variants
        for state in &states.states {
            assert!(impl_content.contains(&format!("    {}({})", state.ident, state.ident)));
        }

        // Verify match statements
        assert!(impl_content.contains("match self {"));
        assert!(impl_content.contains("ActorStates::Create(state) =>"));
        assert!(!impl_content.contains("impl Default")); // No Default implementation
    }
}
