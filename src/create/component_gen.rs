use crate::blox::actor::Actor;
use crate::graph::CodeGenerationGraph;

use super::ToRust;

/// Generates the component definition file for an actor
pub fn generate_component(actor: &Actor) -> String {
    actor.component.to_rust()
}

/// Generates the component definition with graph-based import resolution
pub fn generate_component_with_graph(
    actor: &Actor,
    graph: &CodeGenerationGraph,
) -> Result<String, Box<dyn std::error::Error>> {
    // Find the component module in the graph to get its imports
    let actor_module = actor.ident.to_lowercase();
    let component_module_path = format!("{}::component", actor_module);

    if let Some(component_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&component_module_path)
    {
        let imports = graph.get_imports_for_module(component_module_idx);

        // Generate basic component content
        let basic_content = actor.component.to_rust();

        // Add imports if any were found
        if !imports.is_empty() {
            let imports_section = format!("{}\n\n", imports.join("\n"));
            Ok(format!("{}{}", imports_section, basic_content))
        } else {
            Ok(basic_content)
        }
    } else {
        // Fallback to basic generation if graph lookup fails
        Ok(actor.component.to_rust())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_actor;

    #[test]
    fn test_generate_component() {
        let test_actor = create_test_actor();
        let component_content = generate_component(&test_actor);

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
