use crate::blox::actor::Actor;

use super::ToRust;

/// Generates the component definition file for an actor
pub fn generate_component(actor: &Actor) -> String {
    actor.component.to_rust()
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
