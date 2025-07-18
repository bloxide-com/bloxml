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
        let imports = graph.get_imports_for_module(component_module_idx);

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

/// Generate component with automatic import detection based on component structure analysis
pub fn generate_component_with_structural_analysis(
    actor: &Actor,
    graph: &mut CodeGenGraph,
) -> Result<String, Box<dyn std::error::Error>> {
    let actor_module = actor.ident.to_lowercase();
    let component_module_path = format!("{actor_module}::component");

    // Analyze the component structure directly to determine required imports
    graph.analyze_component_imports(&component_module_path, &actor.component);

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
        let imports = graph.get_imports_for_module(component_module_idx);

        // Add imports if any were found
        if !imports.is_empty() {
            let imports_section = format!("{}\n\n", imports.join("\n"));
            format!("{mod_comment}{imports_section}")
        } else {
            mod_comment
        }
    } else {
        mod_comment
    };

    // Generate the component code
    let component_code = actor.component.to_rust(graph);
    Ok(format!("{header}{component_code}"))
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
        let component_module_path = "testactor::component";
        graph.add_generated_module(component_module_path);

        // Generate component with structural analysis
        let result = generate_component_with_structural_analysis(&actor, &mut graph);
        assert!(result.is_ok());

        let generated_code = result.unwrap();

        // Check that module comment is included
        assert!(generated_code.contains("//! # testactor Components"));

        // Get the detected imports from structural analysis
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical(component_module_path)
            .expect("Component module should exist");
        let imports = graph.get_imports_for_module(component_module_idx);

        // Should always detect Components trait usage (every component implements it)
        assert!(
            imports.iter().any(|imp| imp.contains("Components")),
            "Should always detect Components trait usage. Found imports: {:?}",
            imports
        );

        println!("Structural analysis detected imports: {:#?}", imports);
        println!("Generated code:\n{}", generated_code);
    }

    #[test]
    fn test_structural_vs_text_analysis_comparison() {
        use crate::blox::actor::Actor;
        use crate::tests::create_test_states;
        use std::path::PathBuf;

        // Create a test actor with message handlers to get more interesting imports
        let mut actor = Actor::new(
            "TestActor",
            PathBuf::from("test"),
            create_test_states(),
            None,
        );

        // Add some message handles and receivers to make it more interesting
        actor
            .component
            .message_handles
            .add_handle(MessageHandle::new("test_handle", "TestMessage"));
        actor
            .component
            .message_receivers
            .add_receiver(MessageReceiver::new("test_rx", "TestMessage"));

        // Test structural analysis
        let mut structural_graph = CodeGenGraph::new();
        let component_module_path = "testactor::component";
        structural_graph.add_generated_module(component_module_path);

        // Analyze component structure directly
        structural_graph.analyze_component_imports(component_module_path, &actor.component);

        let component_module_idx = structural_graph
            .graph
            .find_module_by_path_hierarchical(component_module_path)
            .unwrap();
        let structural_imports = structural_graph.get_imports_for_module(component_module_idx);

        // Test text-based analysis for comparison
        let mut text_graph = CodeGenGraph::new();
        text_graph.add_generated_module(component_module_path);

        let mut temp_graph = CodeGenGraph::new();
        let component_code = actor.component.to_rust(&mut temp_graph);
        text_graph.analyze_and_add_imports(component_module_path, &component_code);

        let text_component_module_idx = text_graph
            .graph
            .find_module_by_path_hierarchical(component_module_path)
            .unwrap();
        let text_imports = text_graph.get_imports_for_module(text_component_module_idx);

        println!("Structural analysis imports: {:#?}", structural_imports);
        println!("Text analysis imports: {:#?}", text_imports);

        // Both should detect Components
        assert!(
            structural_imports
                .iter()
                .any(|imp| imp.contains("Components"))
        );
        assert!(text_imports.iter().any(|imp| imp.contains("Components")));

        // Both should detect TokioMessageHandle since we added handles
        assert!(
            structural_imports
                .iter()
                .any(|imp| imp.contains("TokioMessageHandle"))
        );
        assert!(
            text_imports
                .iter()
                .any(|imp| imp.contains("TokioMessageHandle"))
        );

        // The structural analysis should be more reliable and consistent
        // (this test demonstrates that both approaches work, but structural is cleaner)
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
                "Expected {} for type '{}' in code: '{}'",
                expected, type_name, code
            );
        }
    }

    #[test]
    fn test_structural_analysis_benefits() {
        // Create a component with specific structure
        let mut component = Component::new(
            "TestComponents".to_string(),
            MessageHandles::new("TestHandles"),
            MessageReceivers::new("TestReceivers"),
            create_test_states(),
            None,
            ExtState::default(),
        );

        // Add a message handle - this should trigger TokioMessageHandle import
        component
            .message_handles
            .add_handle(MessageHandle::new("my_handle", "MyMessage"));

        // Add a message receiver - this should trigger Runtime/MessageSender imports
        component
            .message_receivers
            .add_receiver(MessageReceiver::new("my_rx", "MyMessage"));

        // Test structural analysis
        let mut structural_graph = CodeGenGraph::new();
        structural_graph.add_generated_module("test::component");
        structural_graph.analyze_component_imports("test::component", &component);

        // Get what imports were detected using the proper API
        let component_module_idx = structural_graph
            .graph
            .find_module_by_path_hierarchical("test::component")
            .expect("Component module should exist");
        let imports = structural_graph.get_imports_for_module(component_module_idx);

        // Should detect exactly what we need based on component structure
        assert!(imports.iter().any(|imp| imp.contains("Components")));
        assert!(imports.iter().any(|imp| imp.contains("TokioMessageHandle")));
        assert!(imports.iter().any(|imp| imp.contains("Runtime")));
        assert!(imports.iter().any(|imp| imp.contains("MessageSender")));

        // Benefits of structural analysis:
        // ✅ Analyzes actual data structures
        // ✅ No string parsing needed
        // ✅ More reliable and maintainable
        // ✅ Faster execution
        // ✅ Type-safe analysis
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

        // First populate basic graph structure
        graph.populate_from_actor(&actor).unwrap();

        // Then analyze all modules structurally
        graph.analyze_all_module_imports(&actor);

        // Check component imports using proper API
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::component")
            .expect("Component module should exist");
        let component_imports = graph.get_imports_for_module(component_module_idx);

        assert!(
            component_imports
                .iter()
                .any(|imp| imp.contains("Components"))
        );
        assert!(
            component_imports
                .iter()
                .any(|imp| imp.contains("TokioMessageHandle"))
        );
        assert!(component_imports.iter().any(|imp| imp.contains("Runtime")));

        // Check states imports using proper API
        let states_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::states")
            .expect("States module should exist");
        let states_imports = graph.get_imports_for_module(states_module_idx);

        assert!(
            states_imports
                .iter()
                .any(|imp| imp.contains("StateMachine"))
        );
        assert!(states_imports.iter().any(|imp| imp.contains("State")));

        // Check ext_state imports using proper API
        let ext_state_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::ext_state")
            .expect("ExtState module should exist");
        let ext_state_imports = graph.get_imports_for_module(ext_state_module_idx);

        assert!(
            ext_state_imports
                .iter()
                .any(|imp| imp.contains("ExtendedState"))
        );

        // Check runtime imports using proper API
        let runtime_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("testactor::runtime")
            .expect("Runtime module should exist");
        let runtime_imports = graph.get_imports_for_module(runtime_module_idx);

        assert!(runtime_imports.iter().any(|imp| imp.contains("Runnable")));

        println!("✅ All module imports detected correctly through structural analysis!");
    }
}
