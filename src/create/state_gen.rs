use crate::{
    actor::Actor,
    blox::state::{State, States},
};
use std::error::Error;

/// Generate a state implementation for a specific State in the States collection
pub fn generate_inner_states(
    state: &State,
    parent_enum: &States,
) -> Result<String, Box<dyn Error>> {
    let state_name = &state.ident;

    let parent_relation = if let Some(parent) = &state.parent {
        format!("{}::{parent}({parent})", parent_enum.state_enum.get().ident,)
    } else {
        format!("{}::Uninit(Uninit)", parent_enum.state_enum.get().ident)
    };

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
    );
    Ok(impl_content)
}

/// Generate a unified StateEnum implementation that contains all states
pub fn generate_state_enum_impl(actor: &Actor) -> Result<String, Box<dyn Error>> {
    let states = &actor.component.states;
    let enum_name = states.state_enum.get().ident.clone();

    let imports = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}use {ident_lowercase}::{ident};\n",
            ident_lowercase = state.ident.to_lowercase(),
            ident = state.ident
        )
    });

    let variants = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}    /// {state_name} state\n    {state_name}({state_name}),\n",
            state_name = state.ident
        )
    });

    let handle_message_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!("{acc}            {enum_name}::{state_name}(state) => state.handle_message(state_machine, message),\n", 
            state_name = state.ident
        )
    });

    let on_entry_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_entry(state_machine),\n",
            state_name = state.ident
        )
    });

    let on_exit_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_exit(state_machine),\n",
            state_name = state.ident
        )
    });

    let parent_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.parent(),\n",
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
        message: {message_set},
    ) -> Option<Transition<Components::States, {message_set}>> {{
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
    fn parent(&self) -> {enum_name} {{
        match self {{
{parent_arms}
        }}
    }}
}}
"#,
        message_set = actor.message_set_ident(),
    );

    Ok(impl_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blox::{
            enums::EnumDef,
            state::{State, StateEnum, States},
        },
        tests::create_test_actor,
    };

    #[test]
    fn test_generate_state_impls() {
        let state = State::from("Create");

        let states = States::new(
            vec![state.clone()],
            StateEnum::new(EnumDef::new("ActorStates", vec![])),
        );
        let impl_content =
            generate_inner_states(&state, &states).expect("Failed to generate state impls");
        let ident = state.ident;
        eprintln!("State impl for {ident}: {impl_content}");

        assert!(impl_content.contains(&format!("pub struct {ident}")));
        assert!(impl_content.contains(&format!("impl State<Components> for {ident}")));
    }

    #[test]
    fn test_generate_state_enum_impl() {
        let mut actor = create_test_actor();
        let state_enum = StateEnum::new(EnumDef::new("ActorStates", vec![]));

        let states = States::new(
            vec![
                State::from("Create"),
                State::new("Update", Some("Create".to_string()), None),
                State::new("Delete", Some("Update".to_string()), None),
            ],
            state_enum,
        );

        actor.component.states = states;

        let impl_content =
            generate_state_enum_impl(&actor).expect("Failed to generate state enum impl");
        eprintln!("State enum impl: {}", impl_content);

        assert!(impl_content.contains("pub enum ActorStates"));
        assert!(impl_content.contains("impl State<Components> for ActorStates"));

        for state in &actor.component.states.states {
            assert!(impl_content.contains(&format!("    {}({})", state.ident, state.ident)));
        }

        assert!(impl_content.contains("match self {"));
        assert!(impl_content.contains("ActorStates::Create(state) =>"));
    }
}
