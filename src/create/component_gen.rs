use crate::blox::actor::Actor;
use crate::graph::CodeGenGraph;

use super::ToRust;

/// Generates the component definition file for an actor
pub fn generate_component(actor: &Actor) -> String {
    let mut graph = CodeGenGraph::new();
    actor.component.to_rust(&mut graph)
}

/// Generates the component definition with graph-based import resolution
pub fn generate_component_with_graph(
    actor: &Actor,
    graph: &mut CodeGenGraph,
) -> Result<String, Box<dyn std::error::Error>> {
    // Find the component module in the graph to get its imports
    let actor_module = actor.ident.to_lowercase();
    let component_module_path = format!("{actor_module}::component");
    let mod_comment = format!(
        r"//! # {actor_module} Components
//!
//! This module defines the component structure for the {actor_module} Blox.
//! It specifies the states, message types, extended state, and communication
//! channels that make up the {actor_module} component.
"
    );

    let header = if let Some(component_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&component_module_path)
    {
        let imports = graph
            .get_imports_for_module(component_module_idx)
            .collect::<Vec<_>>();

        // Add imports if any were found
        if !imports.is_empty() {
            let imports_section = format!("{}\n\n", imports.join("\n"));
            format!("{mod_comment}{imports_section}")
        } else {
            mod_comment
        }
    } else {
        String::new()
    };

    Ok(format!("{header}\n\n{}", actor.component.to_rust(graph)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blox::{
            component::Component,
            ext_state::ExtState,
            message_handlers::{MessageHandle, MessageHandles, MessageReceiver, MessageReceivers},
        },
        graph::CodeGenGraph,
        tests::create_test_states,
    };

    #[test]
    fn test_to_rust() {
        let mut handles = MessageHandles::new("ActorHandles");
        handles.add_handle(MessageHandle::new("test_handle", "TestMessage"));

        let mut receivers = MessageReceivers::new("ActorReceivers");
        receivers.add_receiver(MessageReceiver::new("test_rx", "TestMessage"));

        let component = Component::new(
            "ActorComponents".to_string(),
            handles,
            receivers,
            create_test_states(),
            None,
            ExtState::default(),
        );
        let mut graph = CodeGenGraph::new();
        let rust_code = component.to_rust(&mut graph);

        assert!(rust_code.contains("pub struct ActorHandles"));
        assert!(rust_code.contains("pub struct ActorReceivers"));
        assert!(rust_code.contains("pub test_handle: TokioMessageHandle<TestMessage>"));
        assert!(rust_code.contains("pub test_rx: <<TokioRuntime as Runtime>::MessageHandle<TestMessage> as MessageSender>::ReceiverType"));
    }

    #[test]
    fn test_structural_import_detection() {
        use crate::blox::actor::Actor;
        use crate::tests::create_test_states;
        use std::path::PathBuf;

        // Create a test actor
        let actor = Actor::new(
            "TestActor",
            PathBuf::from("test"),
            create_test_states(),
            None,
        );

        // Create a graph and add the component module
        let mut graph = CodeGenGraph::new();
        graph
            .analyze_actor(&actor)
            .expect("Actor analysis should succeed");

        let component_module_path = "testactor::component";

        // Generate component with the pre-populated graph
        let result = generate_component_with_graph(&actor, &mut graph);
        assert!(result.is_ok());

        let generated_code = result.unwrap();

        // Check that module comment is included
        assert!(generated_code.contains("//! # testactor Components"));

        // Get the detected imports from structural analysis
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical(component_module_path)
            .expect("Component module should exist");
        let imports = graph
            .get_imports_for_module(component_module_idx)
            .collect::<Vec<_>>();

        // Should always detect Components trait usage (every component implements it)
        assert!(imports.iter().any(|s| s.contains("Components")),);
    }

    #[test]
    fn test_code_usage_detection() {
        let graph = CodeGenGraph::new();

        // Test various code patterns
        let test_cases = vec![
            ("impl Components for Test", "Components", true),
            (
                "pub field: TokioMessageHandle<Msg>",
                "TokioMessageHandle",
                true,
            ),
            (": TokioRuntime", "TokioRuntime", true),
            ("<<Runtime>::MessageHandle", "Runtime", true),
            ("as MessageSender", "MessageSender", true),
            ("random_function()", "Components", false),
            ("SomeOtherType", "TokioMessageHandle", false),
        ];

        for (code, type_name, expected) in test_cases {
            let result = graph.code_uses_type(code, type_name);
            assert_eq!(
                result, expected,
                "Expected {expected} for type '{type_name}' in code: '{code}'"
            );
        }
    }

    #[test]
    fn test_comprehensive_structural_analysis() {
        use crate::blox::actor::Actor;
        use crate::tests::create_test_states;
        use std::path::PathBuf;

        // Create a test actor with comprehensive structure
        let mut actor = Actor::new(
            "TestActor",
            PathBuf::from("test"),
            create_test_states(),
            None,
        );

        // Add handles and receivers to trigger different import requirements
        actor
            .component
            .message_handles
            .add_handle(MessageHandle::new("test_handle", "TestMessage"));
        actor
            .component
            .message_receivers
            .add_receiver(MessageReceiver::new("test_rx", "TestMessage"));

        // Test the comprehensive analysis system
        let mut graph = CodeGenGraph::new();

        // Use the new architecture to analyze the actor
        let _ = graph.analyze_actor(&actor);

        // Check component imports using proper API
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::component")
            .expect("Component module should exist");
        let component_imports = graph
            .get_imports_for_module(component_module_idx)
            .collect::<Vec<_>>();

        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("bloxide_tokio::components::Components"))
        );
        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("bloxide_tokio::components::Runtime"))
        );
        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("bloxide_tokio::TokioMessageHandle"))
        );

        // Check states imports using proper API
        let states_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::states")
            .expect("States module should exist");
        let states_imports = graph
            .get_imports_for_module(states_module_idx)
            .collect::<Vec<_>>();

        assert!(
            states_imports
                .iter()
                .any(|s| s.contains("bloxide_tokio::state_machine::StateMachine"))
        );
        assert!(
            states_imports
                .iter()
                .any(|s| s.contains("bloxide_tokio::state_machine::State"))
        );

        // Check ext_state imports using proper API
        let ext_state_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::ext_state")
            .expect("ExtState module should exist");
        let ext_state_imports = graph
            .get_imports_for_module(ext_state_module_idx)
            .collect::<Vec<_>>();

        assert!(
            ext_state_imports
                .iter()
                .any(|s| s.contains("bloxide_tokio::state_machine::ExtendedState"))
        );

        // Check runtime imports using proper API
        let runtime_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::runtime")
            .expect("Runtime module should exist");
        let runtime_imports = graph
            .get_imports_for_module(runtime_module_idx)
            .collect::<Vec<_>>();

        assert!(runtime_imports.iter().any(|s| s.contains("Runnable")));
    }
}
