use crate::blox::actor::Actor;
use std::error::Error;

use super::ToRust;

/// Generates the component definition file for an actor
pub fn generate_component(actor: &Actor) -> Result<String, Box<dyn Error>> {
    let actor_name = &actor.ident;
    let states_name = actor.component.states.state_enum.get().ident.clone();
    let message_set_name = actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or_else(|| format!("{actor_name}MessageSet"));

    let handles = actor.component.message_handles.to_rust();
    let receivers = actor.component.message_receivers.to_rust();

    let component_content = format!(
        r#"//! # {actor_name} Components
//!
//! This module defines the component structure for the {actor_name} Blox.
//! It specifies the states, message types, extended state, and communication
//! channels that make up the {actor_name} component.

use crate::blox::{{StandardMessageHandle, StandardMessageRx}};

use super::{{
    ext_state::{actor_name}ExtState,
    messaging::{message_set_name},
    runtime::{{{actor_name}Handle, {actor_name}Rx}},
    states::{states_name},
}};
use bloxide_tokio::{{
    messaging::{{Message, MessageSet, StandardPayload}},
    TokioRuntime,
    }};

/// Defines the structure of the {actor_name} Blox component
pub struct {actor_name}Components;

impl Components for {actor_name}Components {{
    type States = {states_name};
    type MessageSet = {message_set_name};
    type ExtendedState = {actor_name}ExtState;
    type Receivers = {actor_name}Receivers;
    type Handles = {actor_name}Handles;
}}

/// Receiver channels for the {actor_name} component
{receivers}

/// Message handles for sending messages from the {actor_name} component
{handles}
"#,
    );

    Ok(component_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_actor;

    #[test]
    fn test_generate_component() {
        let test_actor = create_test_actor();
        let component_content =
            generate_component(&test_actor).expect("Failed to generate component content");

        assert!(component_content.contains(&format!("pub struct {}", test_actor.ident)));
        assert!(component_content.contains(&format!(
            "impl Components for {}Components",
            test_actor.ident
        )));
        assert!(component_content.contains(&format!(
            "type States = {}",
            test_actor.component.states.state_enum.get().ident
        )));

        // Test that standard handle and receiver fields are included
        // assert!(
        //     component_content.contains("pub standard_handle: StandardMessageHandle<TokioRuntime>")
        // );
        // assert!(component_content.contains("pub std_rx: StandardMessageRx<TokioRuntime>"));

        // Test that message-specific handles and receivers are included (if any)
        if let Some(message_set) = &test_actor.component.message_set {
            for variant in &message_set.get().variants {
                let message_name = variant
                    .args
                    .first()
                    .unwrap()
                    .to_string()
                    .split("::")
                    .last()
                    .unwrap()
                    .to_lowercase();
                let handle_name = format!("{message_name}_handle");
                let rx_name = format!("{message_name}_rx");

                eprintln!("{handle_name}");
                assert!(component_content.contains(&format!("pub {handle_name}:")));
                assert!(component_content.contains(&format!("pub {rx_name}:")));
            }
        }
    }
}
