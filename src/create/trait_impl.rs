use crate::blox::state::State;
use std::error::Error;

pub fn generate_state_impls(state: &State) -> Result<String, Box<dyn Error>> {
    let parent_state = state
        .parent
        .as_ref()
        .map(|p| p.ident.clone())
        .unwrap_or_else(|| "Init".to_string());

    let state_name = &state.ident;
    let impl_content = format!(
        r#"
use bloxide_core::{{components::Components, message::MessageSet, state_machine::{{StateMachine, State, Transition}}}};
use log::trace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct {state_name};

impl State<Components> for {state_name} {{
    fn on_entry(&self, _state_machine: &mut StateMachine<Components>) {{
        trace!("State on_entry: {state_name}");
    }}

    fn on_exit(&self, _state_machine: &mut StateMachine<Components>) {{
        trace!("State on_exit: {state_name}");
    }}

    fn parent(&self) -> Components::States {{
        {parent_state}::default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blox::state::State;

    #[test]
    fn test_generate_state_impls() {
        let states = vec![
            State::new("Create"),
            State {
                ident: "Update".to_string(),
                parent: Some(Box::new(State::new("Create"))),
            },
        ];

        for state in states {
            let impl_content =
                generate_state_impls(&state).expect("Failed to generate state impls");

            eprintln!("Impl content: {impl_content}");

            // Verify file contents
            assert!(impl_content.contains(&format!("pub struct {}", state.ident)));
            assert!(impl_content.contains(&format!("impl State<Components> for {}", state.ident)));
        }
    }
}
