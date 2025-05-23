use crate::blox::actor::Actor;
use crate::blox::message_handlers::{MessageHandle, MessageReceiver};
use crate::blox::message_set::MessageSet;
use std::error::Error;

/// Generates the component definition file for an actor
pub fn generate_component(actor: &Actor) -> Result<String, Box<dyn Error>> {
    let actor_name = &actor.ident;
    let states_name = actor.states.state_enum.get().ident.clone();
    let message_set_name = actor
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or_else(|| format!("{actor_name}MessageSet"));

    // Generate message handles and receivers
    let handle_fields = if !actor.message_handles.handles.is_empty() {
        // Use existing handles
        actor
            .message_handles
            .handles
            .iter()
            .map(|handle| format_handle_field(handle, actor_name))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // Generate default handles based on message set
        let handles = get_default_handles(actor_name, &message_set_name, &actor.message_set);
        handles
            .iter()
            .map(|handle| format_handle_field(handle, actor_name))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let receiver_fields = if !actor.message_receivers.receivers.is_empty() {
        // Use existing receivers
        actor
            .message_receivers
            .receivers
            .iter()
            .map(|receiver| format_receiver_field(receiver, actor_name))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // Generate default receivers based on message set
        let receivers = get_default_receivers(actor_name, &message_set_name, &actor.message_set);
        receivers
            .iter()
            .map(|receiver| format_receiver_field(receiver, actor_name))
            .collect::<Vec<_>>()
            .join("\n")
    };

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
pub struct {actor_name}Receivers {{
{receiver_fields}
}}

/// Message handles for sending messages from the {actor_name} component
pub struct {actor_name}Handles {{
{handle_fields}
}}
"#,
    );

    Ok(component_content)
}

// Helper function to get default message handles
fn get_default_handles(
    actor_name: &str,
    message_set_name: &str,
    message_set: &Option<MessageSet>,
) -> Vec<MessageHandle> {
    let mut handles = Vec::new();

    // Add standard message handle
    let standard_handle = MessageHandle::standard("standard_handle");
    handles.push(standard_handle);

    // Add actor-specific message handle
    let actor_handle = MessageHandle::new(
        format!("{}_handle", actor_name.to_lowercase()),
        message_set_name.to_string(),
    );
    handles.push(actor_handle);

    // Add handles for each message in the message set
    if let Some(msg_set) = message_set {
        for variant in &msg_set.get().variants {
            // Generate a handle for each message variant if not already added
            assert!(variant.args.len() == 1);
            let message_type = variant.args.first().unwrap().to_string();
            let handle_name = format!(
                "{}_handle",
                message_type.split("::").last().unwrap().to_lowercase()
            );

            // Skip if we already have a similar handle
            if handles.iter().any(|h| h.name == handle_name) {
                continue;
            }

            let handle = MessageHandle::new(handle_name, message_type.to_string());
            handles.push(handle);
        }
    }

    handles
}

// Helper function to get default message receivers
fn get_default_receivers(
    _actor_name: &str,
    _message_set_name: &str,
    message_set: &Option<MessageSet>,
) -> Vec<MessageReceiver> {
    let mut receivers: Vec<MessageReceiver> = Vec::new();

    // Add receivers for each message in the message set
    if let Some(msg_set) = message_set {
        for variant in &msg_set.get().variants {
            // Generate a receiver for each message variant if not already added
            let message_type = variant.args.first().unwrap().to_string();
            let receiver_name = format!(
                "{}_rx",
                message_type.split("::").last().unwrap().to_lowercase()
            );

            // Skip if we already have a similar receiver
            if receivers.iter().any(|r| r.name == receiver_name) {
                continue;
            }

            let receiver = MessageReceiver::new(receiver_name, message_type.to_string());
            receivers.push(receiver);
        }
    }

    receivers
}

// Helper function to format a handle field
fn format_handle_field(handle: &MessageHandle, actor_name: &str) -> String {
    let name = &handle.name;

    // Extract the message type identifier from the handle name
    let message_identifier = if name.contains('_') {
        let parts: Vec<&str> = name.split('_').collect();
        if !parts.is_empty() {
            parts[0].to_string()
        } else {
            actor_name.to_lowercase()
        }
    } else {
        actor_name.to_lowercase()
    };

    let comment = format!("/// Handle for sending {message_identifier}-specific messages");

    // Determine the proper type for the handle
    let handle_type = if handle.message_type.contains("::") {
        // If it's a fully qualified path, use it directly
        format!("TokioMessageHandle<{}>", handle.message_type)
    } else {
        // If it's a simple name, assume it's from the messaging module
        format!("TokioMessageHandle<{}>", handle.message_type)
    };

    format!("    {comment}\n    pub {name}: {handle_type},")
}

// Helper function to format a receiver field
fn format_receiver_field(receiver: &MessageReceiver, actor_name: &str) -> String {
    let name = &receiver.name;

    // Extract the message type identifier from the receiver name
    let message_identifier = if name.contains('_') {
        let parts: Vec<&str> = name.split('_').collect();
        if !parts.is_empty() {
            parts[0].to_string()
        } else {
            actor_name.to_lowercase()
        }
    } else {
        actor_name.to_lowercase()
    };

    let comment = format!("/// Receiver for {message_identifier}-specific messages");

    // Determine the proper type for the receiver
    let receiver_type = format!("Receiver<{}>", receiver.message_type);

    format!("    {comment}\n    pub {name}: {receiver_type},")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_test_actor;

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
            test_actor.states.state_enum.get().ident
        )));

        // Test that standard handle and receiver fields are included
        // assert!(
        //     component_content.contains("pub standard_handle: StandardMessageHandle<TokioRuntime>")
        // );
        // assert!(component_content.contains("pub std_rx: StandardMessageRx<TokioRuntime>"));

        // Test that message-specific handles and receivers are included (if any)
        if let Some(message_set) = &test_actor.message_set {
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

                assert!(
                    component_content
                        .to_lowercase()
                        .contains(&format!("pub {handle_name}:"))
                );
                assert!(
                    component_content
                        .to_lowercase()
                        .contains(&format!("pub {rx_name}:"))
                );
            }
        }
    }
}
