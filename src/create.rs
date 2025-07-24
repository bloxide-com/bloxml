mod file_gen;
mod state_gen;

pub use file_gen::*;
pub use state_gen::*;

use crate::blox::actor::Actor;
use crate::blox::state::State;
use crate::graph::CodeGenGraph;
use std::{
    error::Error,
    fs::{self, File},
    path::Path,
};

pub trait ToRust {
    fn to_rust(&self, generator: &ActorGenerator) -> String;
}

/// Unified generator for all actor-related code generation
pub struct ActorGenerator {
    graph: CodeGenGraph,
    actor: Actor,
}

impl ActorGenerator {
    /// Creates a new ActorGenerator for the given actor.
    pub fn new(actor: Actor) -> Result<Self, Box<dyn Error>> {
        let mut generator = Self {
            graph: CodeGenGraph::new(),
            actor,
        };
        generator.graph.analyze_actor(&generator.actor)?;
        Ok(generator)
    }

    /// Gets a reference to the actor
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// Gets a reference to the internal graph
    pub fn graph(&self) -> &CodeGenGraph {
        &self.graph
    }

    /// Gets a mutable reference to the internal graph
    pub fn graph_mut(&mut self) -> &mut CodeGenGraph {
        &mut self.graph
    }

    /// Gets the component type name for this actor
    pub fn component_type(&self) -> &str {
        &self.actor.component.ident
    }

    /// Gets the message set name for this actor
    pub fn message_set(&self) -> String {
        self.actor
            .component
            .message_set
            .as_ref()
            .map(|ms| ms.get().ident.clone())
            .unwrap_or_else(|| format!("<{} as Components>::MessageSet", self.component_type()))
    }

    /// Gets the actor module name (lowercase)
    pub fn actor_module(&self) -> String {
        self.actor.ident.to_lowercase()
    }

    /// Generates the component definition
    pub fn generate_component(&mut self) -> Result<String, Box<dyn Error>> {
        let actor_module = self.actor.ident.to_lowercase();
        let component_module_path = format!("{actor_module}::component");
        let mod_comment = format!(
            r"//! # {actor_module} Components
//!
//! This module defines the component structure for the {actor_module} Blox.
//! It specifies the states, message types, extended state, and communication
//! channels that make up the {actor_module} component.
"
        );

        let component_module_idx = self
            .graph
            .graph
            .find_module_by_path_hierarchical(&component_module_path)
            .expect("Component module should exist after analysis");
        let imports = self
            .graph
            .get_imports_for_module(component_module_idx)
            .collect::<Vec<_>>();

        let header = if !imports.is_empty() {
            let imports_section = format!("{}\n\n", imports.join("\n"));
            format!("{mod_comment}{imports_section}")
        } else {
            mod_comment
        };

        Ok(format!(
            "{header}\n\n{}",
            self.actor.component.to_rust(self)
        ))
    }

    /// Generates the message set module
    pub fn generate_messaging(&mut self) -> Result<Option<String>, Box<dyn Error>> {
        let Some(message_set) = &self.actor.component.message_set else {
            return Ok(None);
        };

        let enum_def = message_set.get();
        let actor_module = self.actor.ident.to_lowercase();

        let messaging_module_path = format!("{actor_module}::messaging");
        let messaging_module_idx = self
            .graph
            .graph
            .find_module_by_path_hierarchical(&messaging_module_path)
            .expect("Messaging module should exist after analysis");
        let imports = self
            .graph
            .get_imports_for_module(messaging_module_idx)
            .collect::<Vec<_>>();

        let imports_section = if imports.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", imports.join("\n"))
        };

        let custom_types = message_set
            .custom_types
            .iter()
            .map(|enum_def| self.generate_custom_type_definition(enum_def))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n\n");

        let enum_definition = self.generate_enum_definition(enum_def)?;

        let content = format!(
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

impl MessageSet for {ident} {{}}
"#,
            ident = enum_def.ident,
        );

        Ok(Some(content))
    }

    /// Generates the runtime module
    pub fn generate_runtime(&self) -> Result<String, Box<dyn Error>> {
        let actor_name = &self.actor.ident;
        let actor_module = self.actor.ident.to_lowercase();

        let runtime_module_path = format!("{actor_module}::runtime");
        let runtime_module_idx = self
            .graph
            .graph
            .find_module_by_path_hierarchical(&runtime_module_path)
            .expect("Runtime module should exist after analysis");
        let imports = self
            .graph
            .get_imports_for_module(runtime_module_idx)
            .collect::<Vec<_>>();

        let imports_section = if imports.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", imports.join("\n"))
        };

        let message_set_name = self
            .actor
            .component
            .message_set
            .as_ref()
            .map(|ms| ms.get().ident.clone())
            .unwrap_or_default();

        let mut select_arms = String::new();
        if let Some(message_set) = &self.actor.component.message_set {
            let iter = self
                .actor
                .component
                .message_receivers
                .receivers
                .clone()
                .into_iter()
                .zip(message_set.get().variants.clone());

            for (receiver, variant) in iter {
                select_arms.push_str(&format!(
                    r#"                    Some(msg) = self.receivers.{ident}.recv() => {{
                        let current_state = self.state_machine.current_state.clone();
                        self.state_machine.dispatch({message_set_name}::{variant_name}(msg), &current_state);
                    }}
"#,
                    ident = receiver.ident,
                    variant_name = variant.ident
                ));
            }
        }

        let states = &self.actor.component.states;
        let first_state = &states.states[0];
        let second_state = states.states.get(1).unwrap_or(&states.states[0]);
        let state_enum_name = &states.state_enum.get().ident;

        let content = format!(
            r#"{imports_section}use super::{{
    component::{actor_name}Components,
    states::{{
        {first_state_lower}::{first_state},
        {second_state_lower}::{second_state},
        {state_enum_name},
    }},
    messaging::{message_set_name},
}};

impl Runnable<{actor_name}Components> for Blox<{actor_name}Components> {{
    fn run(mut self: Box<Self>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {{
        self.state_machine.init(
            &{state_enum_name}::{first_state}({first_state}),
            &{state_enum_name}::{second_state}({second_state}),
        );

        Box::pin(async move {{
            loop {{
                select! {{
{select_arms}
                }}
            }}
        }})
    }}
}}"#,
            first_state = first_state.ident,
            first_state_lower = first_state.ident.to_lowercase(),
            second_state = second_state.ident,
            second_state_lower = second_state.ident.to_lowercase(),
        );

        Ok(content)
    }

    /// Generates the extended state module
    pub fn generate_ext_state(&mut self) -> String {
        let ident = &self.actor.ident;
        format!(
            r#"//! # {ident} Extended State
//! 
//! Extended state for the {ident} component.
//! This file defines the extended state data structure that persists across state transitions.

/// Extended state for the {ident} component
{ext_state}
"#,
            ext_state = self.actor.component.ext_state.to_rust(self),
        )
    }

    /// Generates individual state implementations using ToRust
    pub fn generate_state_impl(&self, state: &State) -> Result<String, Box<dyn Error>> {
        let actor_mod = self.actor_module();
        let state_module_path = format!("{actor_mod}::states::{}", state.ident.to_lowercase());
        let state_module_idx = self
            .graph
            .graph
            .find_module_by_path_hierarchical(&state_module_path)
            .expect("State module should exist after analysis");
        let imports = self
            .graph
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>();

        let imports_section = if imports.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", imports.join("\n"))
        };

        // Use ToRust trait directly
        let state_code = state.to_rust(self);

        Ok(format!("{imports_section}{state_code}"))
    }

    /// Generates the state enum implementation using ToRust
    pub fn generate_state_enum(&self) -> Result<String, Box<dyn Error>> {
        let actor_mod = self.actor_module();
        let state_module_path = format!("{actor_mod}::states");
        let state_module_idx = self
            .graph
            .graph
            .find_module_by_path_hierarchical(&state_module_path)
            .expect("States module should exist after analysis");
        let imports = self
            .graph
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>();

        let imports_section = if imports.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", imports.join("\n"))
        };

        // Use ToRust trait directly
        let state_enum_code = self.actor.component.states.state_enum.to_rust(self);

        Ok(format!("{imports_section}{state_enum_code}"))
    }

    /// Generates all files for the actor module
    pub fn generate_all_files(&mut self) -> Result<(), Box<dyn Error>> {
        // Validate states first
        self.actor.component.states.validate()?;

        let mod_path = self.actor.create_mod_path();
        self.create_module_dir(&mod_path)?;

        // Generate all module files
        let modules = ["messaging.rs", "ext_state.rs", "component.rs", "runtime.rs"];
        self.create_module_files(&mod_path, &modules)?;

        // Generate messaging module if message set exists
        if let Some(messaging_content) = self.generate_messaging()? {
            fs::write(mod_path.join("messaging.rs"), messaging_content)?;
        }

        // Generate component.rs
        let component_content = self.generate_component()?;
        fs::write(mod_path.join("component.rs"), component_content)?;

        // Generate ext_state.rs
        let ext_state_content = self.generate_ext_state();
        fs::write(mod_path.join("ext_state.rs"), ext_state_content)?;

        // Generate runtime.rs
        let runtime_content = self.generate_runtime()?;
        fs::write(mod_path.join("runtime.rs"), runtime_content)?;

        // Generate states module
        self.generate_states_module(&mod_path.join("states"))?;

        // Create root mod.rs
        let mut all_modules = modules
            .iter()
            .map(|m| m.trim_end_matches(".rs"))
            .collect::<Vec<_>>();
        all_modules.push("states");
        self.create_root_mod_rs(&mod_path, &all_modules)?;

        Ok(())
    }

    // Helper methods for file operations
    fn create_module_dir(&self, path: &Path) -> Result<(), String> {
        fs::create_dir_all(path)
            .map_err(|e| format!("Error creating directory {}: {e}", path.display()))
    }

    fn create_module_files(&self, mod_path: &Path, modules: &[&str]) -> Result<(), Box<dyn Error>> {
        modules
            .iter()
            .map(|mod_file| mod_path.join(mod_file))
            .map(File::create)
            .try_for_each(|res| {
                res.map(|_| ())
                    .map_err(|e| format!("Error creating file: {e}").into())
            })
    }

    fn create_root_mod_rs(&self, mod_path: &Path, modules: &[&str]) -> Result<(), Box<dyn Error>> {
        let mod_rs_content = modules
            .iter()
            .map(|mod_name| format!("pub mod {mod_name};"))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(mod_path.join("mod.rs"), mod_rs_content)
            .map_err(|e| format!("Error creating mod.rs file: {e}").into())
    }

    fn generate_states_module(&self, states_path: &Path) -> Result<(), Box<dyn Error>> {
        self.create_module_dir(states_path)?;

        // Generate individual state files
        for state in &self.actor.component.states.states {
            let state_content = self.generate_state_impl(state)?;
            let state_file = states_path.join(format!("{}.rs", state.ident.to_lowercase()));
            fs::write(state_file, state_content)?;
        }

        // Generate states/mod.rs with state modules and enum
        let state_modules = self
            .actor
            .component
            .states
            .states
            .iter()
            .map(|state| format!("pub mod {};", state.ident.to_lowercase()))
            .collect::<Vec<_>>()
            .join("\n");

        let state_enum_impl = self.generate_state_enum()?;

        let mod_rs_content = format!("{}\n\n{}", state_modules, state_enum_impl);
        fs::write(states_path.join("mod.rs"), mod_rs_content)?;

        Ok(())
    }

    // Helper methods for message generation
    fn generate_enum_definition(
        &self,
        enum_def: &crate::blox::enums::EnumDef,
    ) -> Result<String, Box<dyn Error>> {
        let enum_name = &enum_def.ident;

        let variants = enum_def
            .variants
            .iter()
            .fold(String::new(), |acc, variant| {
                if variant.args.is_empty() {
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

    fn generate_custom_type_definition(
        &self,
        enum_def: &crate::blox::enums::EnumDef,
    ) -> Result<String, Box<dyn Error>> {
        let enum_name = &enum_def.ident;

        let variants = enum_def
            .variants
            .iter()
            .fold(String::new(), |acc, variant| {
                if variant.args.is_empty() {
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
            r#"/// Custom type definition
#[derive(Debug, Clone, PartialEq)]
pub enum {enum_name} {{
{variants}}}"#
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_actor;

    #[test]
    fn test_actor_generator_basic() {
        let actor = create_test_actor();
        let mut generator = ActorGenerator::new(actor).expect("Generator creation should succeed");

        let component_result = generator.generate_component();
        assert!(component_result.is_ok());
        let component_code = component_result.unwrap();
        assert!(component_code.contains("pub struct ActorComponents"));
        assert!(component_code.contains("impl Components for ActorComponents"));

        // Test messaging generation
        let messaging_result = generator.generate_messaging();
        assert!(messaging_result.is_ok());
        let messaging_code = messaging_result.unwrap();
        assert!(messaging_code.is_some());
        assert!(messaging_code.unwrap().contains("pub enum ActorMessageSet"));

        // Test runtime generation
        let runtime_result = generator.generate_runtime();
        assert!(runtime_result.is_ok());
        let runtime_code = runtime_result.unwrap();
        assert!(runtime_code.contains("impl Runnable<ActorComponents>"));

        // Test ext_state generation
        let ext_state_code = generator.generate_ext_state();
        assert!(ext_state_code.contains("Extended state for the Actor component"));
    }

    #[test]
    fn test_actor_generator_state_generation() {
        let actor = create_test_actor();
        let generator = ActorGenerator::new(actor).expect("Generator creation should succeed");

        // Test individual state generation (analysis already happened during creation)
        let create_state = &generator.actor().component.states.states[0];
        let state_result = generator.generate_state_impl(create_state);
        assert!(state_result.is_ok());
        let state_code = state_result.unwrap();
        assert!(state_code.contains("pub struct Create"));
        assert!(state_code.contains("impl State<ActorComponents> for Create"));

        // Test state enum generation
        let state_enum_result = generator.generate_state_enum();
        assert!(state_enum_result.is_ok());
        let state_enum_code = state_enum_result.unwrap();
        assert!(state_enum_code.contains("pub enum ActorStates"));
        assert!(state_enum_code.contains("impl State<ActorComponents> for ActorStates"));

        // Verify that state types are available as imports from the graph
        let actor_mod = generator.actor_module();
        let state_module_path = format!("{actor_mod}::states");
        let state_module_idx = generator
            .graph()
            .graph
            .find_module_by_path_hierarchical(&state_module_path)
            .unwrap();
        let imports = generator
            .graph()
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>();

        // Verify that state types are imported (for StateEnum)
        for state in &generator.actor().component.states.states {
            assert!(
                imports.iter().any(|imp| imp.contains(&state.ident)),
                "Should import state type {}",
                state.ident
            );
        }
    }

    #[test]
    fn test_actor_generator_creation() {
        let actor = create_test_actor();

        let mut generator = ActorGenerator::new(actor).expect("Generator creation should succeed");

        let component_result = generator.generate_component();
        assert!(component_result.is_ok());
    }
}
