use crate::{blox::enums::EnumDef, blox::message_set::MessageSet};
use std::error::Error;

/// Generates Rust code for a message set based on the provided MessageSet configuration.
///
/// # Arguments
/// * `msg_set` - The message set configuration containing the message enum and variants
///
/// # Returns
/// A `Result` containing the generated Rust code as a `String` or an error
pub fn generate_message_set(msg_set: &MessageSet) -> Result<String, Box<dyn Error>> {
    // Access the wrapped MsgEnum
    let enum_def = msg_set.get();

    let mut output = format!(
        r#"//! # {ident} Message Module
//!
//! This module defines the message types and payloads used for communication
//! within the system. The message set follows a hierarchical structure.
//!
//! ## Message Structure
//! - `MessageSet` - The top-level message set enum that wraps all message types

use serde::{{Deserialize, Serialize}};
use bloxide_tokio::messaging::{{Message, MessageSet}};

{enum_definition}
"#,
        ident = enum_def.ident,
        enum_definition = generate_enum_definition(enum_def)?
    );

    output.push_str(&format!("\nimpl MessageSet for {} {{}}", enum_def.ident));

    Ok(output)
}

/// Generates the message enum with all variants from the MsgEnum
fn generate_enum_definition(enum_def: &EnumDef) -> Result<String, Box<dyn Error>> {
    let enum_name = &enum_def.ident;

    // Generate variants directly from the enum_def.variants
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
                // Tuple variant with args
                let args = variant
                    .args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(", ");

                format!(
                    "{acc}    /// {ident}\n    {ident}({args}),\n",
                    ident = variant.ident,
                    args = args
                )
            }
        });

    Ok(format!(
        r#"/// The primary message set for the actor's state machine.
///
/// This enum contains all possible message types that can be dispatched to the
/// actor's state machine, allowing for unified message processing logic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum {enum_name} {{
{variants}}}"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blox::enums::{EnumDef, EnumVariant, Link};

    #[test]
    fn test_generate_message_set() {
        // Create a MessageSet with a MsgEnum with one variant having an arg
        let enum_def = EnumDef::new(
            "TestMessageSet",
            vec![
                EnumVariant {
                    ident: "Variant1".to_string(),
                    args: vec![Link::new("SomeType")],
                },
                EnumVariant::new("Variant2", vec![]),
            ],
        );
        let message_set = MessageSet::new(enum_def);

        let result = generate_message_set(&message_set).expect("Failed to generate message set");

        // Verify the generated code contains expected patterns
        assert!(result.contains("pub enum TestMessageSet"));
        assert!(result.contains("Variant1(SomeType)"));
        assert!(result.contains("Variant2,"));
        assert!(result.contains("impl MessageSet for TestMessageSet"));
    }
}
