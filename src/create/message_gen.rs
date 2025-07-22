use crate::blox::actor::Actor;
use crate::graph::CodeGenGraph;
use crate::{blox::enums::EnumDef, blox::message_set::MessageSet};
use std::error::Error;

/// Generates Rust code for a message set based on the provided MessageSet configuration.
///
/// # Arguments
/// * `msg_set` - The message set configuration containing the message enum and variants
///
/// # Returns
/// A `Result` containing the generated Rust code as a `String` or an error
pub fn generate_message_set(
    msg_set: &MessageSet,
    actor: &Actor,
    graph: &mut CodeGenGraph,
) -> Result<String, Box<dyn Error>> {
    let enum_def = msg_set.get();
    let actor_module = actor.ident.to_lowercase();

    // Get imports from graph for the messaging module
    let messaging_module_path = format!("{actor_module}::messaging");
    let imports = if let Some(messaging_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&messaging_module_path)
    {
        graph
            .get_imports_for_module(messaging_module_idx)
            .collect::<Vec<_>>()
    } else {
        // Fallback to hardcoded imports
        vec!["use bloxide_tokio::messaging::{Message, MessageSet};".to_string()]
    };

    let imports_section = if imports.is_empty() {
        String::new()
    } else {
        format!("{};\n\n", imports.join(";\n"))
    };

    let mut output = format!(
        r#"//! # {ident} Message Module
//!
//! This module defines the message types and payloads used for communication
//! within the system. The message set follows a hierarchical structure.
//!
//! ## Message Structure
//! - `MessageSet` - The top-level message set enum that wraps all message types
{imports_section}

{enum_definition}

{custom_types}
"#,
        ident = enum_def.ident,
        custom_types = msg_set
            .custom_types
            .iter()
            .map(generate_custom_type_definition)
            .collect::<Result<Vec<_>, _>>()?
            .join("\n\n"),
        enum_definition = generate_enum_definition(enum_def)?
    );

    output.push_str(&format!("\nimpl MessageSet for {} {{}}", enum_def.ident));

    Ok(output)
}

/// Generates the message enum with all variants from the MsgEnum
fn generate_enum_definition(enum_def: &EnumDef) -> Result<String, Box<dyn Error>> {
    let enum_name = &enum_def.ident;

    let variants = enum_def
        .variants
        .iter()
        .fold(String::new(), |acc, variant| {
            // Check if the variant has args
            if variant.args.is_empty() {
                // Simple variant without args
                format!(
                    "{acc}    /// {ident}\n    {ident},\n",
                    ident = variant.ident
                )
            } else {
                let args = variant
                    .args
                    .iter()
                    .map(|arg| format!("Message<{arg}>"))
                    .collect::<Vec<String>>()
                    .join(", ");

                format!(
                    "{acc}    /// {ident}\n    {ident}({args}),\n",
                    ident = variant.ident,
                )
            }
        });

    Ok(format!(
        r#"/// The primary message set for the actor's state machine.
///
/// This enum contains all possible message types that can be dispatched to the
/// actor's state machine, allowing for unified message processing logic.
pub enum {enum_name} {{
{variants}}}"#
    ))
}

fn generate_custom_type_definition(enum_def: &EnumDef) -> Result<String, Box<dyn Error>> {
    let enum_name = &enum_def.ident;

    let variants = enum_def
        .variants
        .iter()
        .fold(String::new(), |acc, variant| {
            // Check if the variant has args
            if variant.args.is_empty() {
                // Simple variant without args
                format!(
                    "{acc}    /// {ident}\n    {ident},\n",
                    ident = variant.ident
                )
            } else {
                let args = variant
                    .args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(", ");

                format!(
                    "{acc}    /// {ident}\n    {ident}({args}),\n",
                    ident = variant.ident,
                )
            }
        });

    Ok(format!(
        r#"/// The primary message set for the actor's state machine.
///
/// This enum contains all possible message types that can be dispatched to the
/// actor's state machine, allowing for unified message processing logic.
pub enum {enum_name} {{
{variants}}}"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Link,
        blox::enums::{EnumDef, EnumVariant},
        tests::create_test_actor,
    };

    #[test]
    fn test_generate_message_set() {
        let enum_def = EnumDef::new(
            "TestMessageSet",
            vec![
                EnumVariant {
                    ident: "Variant1".to_string(),
                    args: vec![Link::new("SomeType")],
                },
                EnumVariant::new("Variant2", vec![Link::new("SomeType2")]),
            ],
        );

        let custom_type = EnumDef::new(
            "SomeType",
            vec![
                EnumVariant::new("Value1", vec![Link::new("String")]),
                EnumVariant::new("Value2", vec![Link::new("i32")]),
            ],
        );

        let message_set = MessageSet::with_custom_types(enum_def, vec![custom_type]);

        let mut graph = CodeGenGraph::new();
        let result = generate_message_set(&message_set, &create_test_actor(), &mut graph)
            .expect("Failed to generate message set");

        assert!(result.contains("pub enum TestMessageSet"));
        assert!(result.contains("Variant1(Message<SomeType>)"));
        assert!(result.contains("Variant2(Message<SomeType2>)"));
        assert!(result.contains("impl MessageSet for TestMessageSet"));

        // Check custom type generation
        assert!(result.contains("pub enum SomeType"));
        assert!(result.contains("Value1(String)"));
        assert!(result.contains("Value2(i32)"));
    }
}
