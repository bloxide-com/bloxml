mod node;
mod rgraph;
mod ty;

use std::collections::HashMap;
use std::error::Error;

use petgraph::graph::NodeIndex;
pub use ty::Import;

use crate::blox::actor::Actor;
use crate::blox::component::Component;
use crate::blox::message_set::MessageSet;

use crate::ext_state::ExtState;
use crate::graph::node::{Module, Node, RelatedEntry, Relation};
use crate::graph::rgraph::RustGraph;
use crate::graph::ty::{DiscoveredType, TypeContext, TypeLocation};

/// Code generation specific wrapper around RustGraph
///
/// This provides additional functionality for code generation including
/// import tracking, dependency analysis, and module organization.
pub struct CodeGenGraph {
    pub graph: RustGraph,
    /// Types discovered during analysis phase
    discovered_types: Vec<DiscoveredType>,
    /// Registry of known framework types
    framework_types: HashMap<String, String>,
    /// Types that have been resolved to their locations
    resolved_types: HashMap<String, TypeLocation>,
}

impl Default for CodeGenGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenGraph {
    const PRELUDE_TYPES: &[&str] = &[
        "String", "i32", "u32", "i64", "u64", "bool", "Vec", "Option", "Result", "Box", "Arc", "Rc",
    ];

    const RUNTIME_DEFAULT_IMPORTS: &[&str] = &[
        "bloxide_tokio::components::Runnable",
        "bloxide_tokio::components::Blox",
        "std::pin::Pin",
        "tokio::select",
    ];

    const EXT_STATE_DEFAULT_IMPORTS: &[&str] = &["bloxide_tokio::state_machine::ExtendedState"];

    const COMPONENT_DEFAULT_IMPORTS: &[&str] = &["bloxide_tokio::components::Components"];

    const STATES_DEFAULT_IMPORTS: &[&str] = &[
        "bloxide_tokio::state_machine::StateMachine",
        "bloxide_tokio::state_machine::State",
        "bloxide_tokio::state_machine::StateEnum",
        "bloxide_tokio::state_machine::Transition",
        "bloxide_tokio::components::Components",
    ];

    const MESSAGING_DEFAULT_IMPORTS: &[&str] = &[
        "bloxide_tokio::messaging::Message",
        "bloxide_tokio::messaging::MessageSet",
    ];

    pub fn new() -> Self {
        Self {
            graph: RustGraph::new(),
            discovered_types: Vec::new(),
            framework_types: HashMap::new(),
            resolved_types: HashMap::new(),
        }
    }

    /// Phase 1: Bootstrap all known bloxide framework types
    pub fn bootstrap_bloxide_types(&mut self) {
        enum FType {
            Trait,
            Type,
        }
        use FType::*;

        #[rustfmt::skip]
        const FRAMEWORK_TYPES: [(&str, &str, FType); 15] = [
            // Core component types
            ("Components", "bloxide_tokio::components::Components", Trait),
            ("Runtime", "bloxide_tokio::components::Runtime", Trait),
            ("Runnable", "bloxide_tokio::components::Runnable", Trait),
            // Message handling types
            ("TokioMessageHandle", "bloxide_tokio::TokioMessageHandle", Type),
            ("TokioRuntime", "bloxide_tokio::TokioRuntime", Type),
            ("MessageSender", "bloxide_tokio::messaging::MessageSender", Type),
            ("MessageSet", "bloxide_tokio::messaging::MessageSet", Trait),
            ("Message", "bloxide_tokio::messaging::Message", Type),
            ( "StandardPayload", "bloxide_tokio::messaging::StandardPayload", Type),
            ( "StandardMessage", "bloxide_tokio::messaging::StandardMessage", Type),
            // State machine types
            ("StateMachine", "bloxide_tokio::state_machine::StateMachine", Trait),
            ("State", "bloxide_tokio::state_machine::State", Trait),
            ("StateEnum", "bloxide_tokio::state_machine::StateEnum", Trait),
            ("Transition", "bloxide_tokio::state_machine::Transition", Type),
            ("ExtendedState", "bloxide_tokio::state_machine::ExtendedState", Trait),
        ];

        self.framework_types.reserve(FRAMEWORK_TYPES.len());
        for (type_name, full_path, ftype) in FRAMEWORK_TYPES {
            self.framework_types
                .insert(type_name.to_string(), full_path.to_string());
            // Add the type to the graph
            match ftype {
                Trait => self.graph.add_trait_from_path(full_path),
                Type => self.graph.add_type_from_path(full_path),
            };

            // Mark as resolved
            self.resolved_types.insert(
                type_name.into(),
                TypeLocation::BloxideFramework(full_path.into()),
            );
        }
    }

    /// Phase 2: Discover all types used in the actor
    pub fn discover_actor_types(&mut self, actor: &Actor) -> Result<(), Box<dyn Error>> {
        let actor_module_path = actor.ident.to_lowercase();

        // Create the main actor module structure
        let _ = self.add_generated_module(&actor_module_path);
        let _ = self.add_generated_module(&format!("{actor_module_path}::component"));
        let _ = self.add_generated_module(&format!("{actor_module_path}::states"));
        let _ = self.add_generated_module(&format!("{actor_module_path}::ext_state"));
        let _ = self.add_generated_module(&format!("{actor_module_path}::runtime"));
        let _ = self.add_generated_module(&format!("{actor_module_path}::messaging"));

        // Discover types in each component
        self.discover_extended_state_types(&actor.component.ext_state, &actor_module_path)?;
        self.discover_component_types(&actor.component, &actor_module_path)?;
        self.discover_state_types(&actor.component, &actor_module_path)?;

        if let Some(message_set) = &actor.component.message_set {
            self.discover_message_types(message_set, &actor_module_path)?;
        }

        // Discover runtime dependencies
        self.discover_runtime_types(&actor_module_path);

        Ok(())
    }

    /// Discover types used in runtime module
    fn discover_runtime_types(&mut self, actor_module: &str) {
        let module_path = format!("{actor_module}::runtime");

        Self::RUNTIME_DEFAULT_IMPORTS
            .iter()
            .for_each(|import| self.add_dependency_by_path(&module_path, import));
    }

    /// Discover types used in extended state
    fn discover_extended_state_types(
        &mut self,
        ext_state: &ExtState,
        actor_module: &str,
    ) -> Result<(), Box<dyn Error>> {
        let module_path = format!("{actor_module}::ext_state");

        Self::EXT_STATE_DEFAULT_IMPORTS
            .iter()
            .for_each(|import| self.add_dependency_by_path(&module_path, import));

        for field in ext_state.fields() {
            let field_type = field.ty().as_ref();
            self.discover_type_usage(field_type, &module_path, TypeContext::ExtendedState);
        }

        Ok(())
    }

    /// Discover types used in component
    fn discover_component_types(
        &mut self,
        component: &Component,
        actor_module: &str,
    ) -> Result<(), Box<dyn Error>> {
        let module_path = format!("{actor_module}::component");

        Self::COMPONENT_DEFAULT_IMPORTS
            .iter()
            .for_each(|import| self.add_dependency_by_path(&module_path, import));

        // Add conditional framework dependencies based on component structure
        if !component.message_handles.handles.is_empty() {
            self.add_dependency_by_path(&module_path, "bloxide_tokio::TokioMessageHandle");
        }

        if !component.message_receivers.receivers.is_empty() {
            self.add_dependency_by_path(&module_path, "bloxide_tokio::components::Runtime");
            self.add_dependency_by_path(&module_path, "bloxide_tokio::messaging::MessageSender");
            self.add_dependency_by_path(&module_path, "bloxide_tokio::TokioRuntime");
        }

        if component.message_set.is_some() {
            self.add_dependency_by_path(&module_path, "bloxide_tokio::messaging::MessageSet");
        }

        let states_type_path = format!(
            "crate::{actor_module}::states::{}",
            component.states.state_enum.get().ident
        );
        self.add_dependency_by_path(&module_path, &states_type_path);

        if let Some(message_set) = &component.message_set {
            let message_set_path = format!(
                "crate::{actor_module}::messaging::{}",
                message_set.get().ident
            );
            self.add_dependency_by_path(&module_path, &message_set_path);
        }

        let ext_state_path = format!(
            "crate::{actor_module}::ext_state::{}",
            component.ext_state.ident()
        );
        self.add_dependency_by_path(&module_path, &ext_state_path);

        // Discover types in message handles
        component.message_handles.handles.iter().for_each(|handle| {
            self.discover_type_usage(&handle.message_type, &module_path, TypeContext::Component);
        });

        // Discover types in message receivers
        component
            .message_receivers
            .receivers
            .iter()
            .for_each(|receiver| {
                self.discover_type_usage(
                    &receiver.message_type,
                    &module_path,
                    TypeContext::Component,
                );
            });

        Ok(())
    }

    /// Discover types used in states
    fn discover_state_types(
        &mut self,
        component: &Component,
        actor_module: &str,
    ) -> Result<(), Box<dyn Error>> {
        let module_path = format!("{actor_module}::states");

        Self::STATES_DEFAULT_IMPORTS
            .iter()
            .for_each(|import| self.add_dependency_by_path(&module_path, import));

        let component_type_path = format!("crate::{actor_module}::component::{}", component.ident);
        self.add_dependency_by_path(&module_path, &component_type_path);

        if let Some(message_set) = &component.message_set {
            let message_set_path = format!(
                "crate::{actor_module}::messaging::{}",
                message_set.get().ident
            );
            self.add_dependency_by_path(&module_path, &message_set_path);
        }

        component
            .states
            .states
            .iter()
            .filter_map(|state| state.variants.as_ref())
            .flatten()
            .flat_map(|variant| &variant.args)
            .for_each(|arg| {
                self.discover_type_usage(arg.as_ref(), &module_path, TypeContext::States)
            });

        component
            .states
            .state_enum
            .get()
            .variants
            .iter()
            .flat_map(|variant| &variant.args)
            .for_each(|arg| {
                self.discover_type_usage(arg.as_ref(), &module_path, TypeContext::States)
            });

        Ok(())
    }

    /// Discover types used in message set
    fn discover_message_types(
        &mut self,
        message_set: &MessageSet,
        actor_module: &str,
    ) -> Result<(), Box<dyn Error>> {
        let module_path = format!("{actor_module}::messaging");

        Self::MESSAGING_DEFAULT_IMPORTS
            .iter()
            .for_each(|import| self.add_dependency_by_path(&module_path, import));

        // Discover types in main message set enum variants
        message_set
            .def
            .variants
            .iter()
            .flat_map(|variant| &variant.args)
            .for_each(|arg| {
                self.discover_type_usage(arg.as_ref(), &module_path, TypeContext::MessageSet)
            });

        // Register custom types as actor-local types
        for custom_type in &message_set.custom_types {
            let custom_type_path =
                format!("crate::{actor_module}::messaging::{}", custom_type.ident);
            self.resolved_types.insert(
                custom_type.ident.clone(),
                TypeLocation::ActorCustom(custom_type_path),
            );

            custom_type
                .variants
                .iter()
                .flat_map(|variant| &variant.args)
                .for_each(|arg| {
                    self.discover_type_usage(arg.as_ref(), &module_path, TypeContext::MessageSet)
                });
        }

        Ok(())
    }

    /// Discover a type usage and add it to the discovered types list
    fn discover_type_usage(&mut self, type_string: &str, module_path: &str, context: TypeContext) {
        let types = self.extract_types_from_string(type_string);

        for type_name in types {
            // Skip if already discovered in this context
            if self
                .discovered_types
                .iter()
                .any(|dt| dt.name == type_name && dt.used_in_module == module_path)
            {
                continue;
            }

            self.discovered_types.push(DiscoveredType {
                name: type_name.clone(),
                full_type: type_string.to_string(),
                used_in_module: module_path.to_string(),
                context: context.clone(),
            });
        }
    }

    /// Extract individual type names from a complex type string
    fn extract_types_from_string(&self, type_string: &str) -> Vec<String> {
        let mut types = Vec::new();
        let delimiters = ['<', '>', ',', ' ', '(', ')', '[', ']'];

        let parts = type_string
            .split(&delimiters[..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        for part in parts {
            // Skip builtin types
            if Self::PRELUDE_TYPES.contains(&part) {
                continue;
            }

            if part.contains("::") {
                // Extract the final type name from qualified paths
                if let Some(type_name) = part.split("::").last()
                    && self.is_valid_type_name(type_name)
                {
                    types.push(type_name.to_string());
                }
            } else if self.is_valid_type_name(part) {
                types.push(part.to_string());
            }
        }

        types
    }

    /// Check if a string looks like a valid Rust type name
    fn is_valid_type_name(&self, name: &str) -> bool {
        if name.is_empty() || name.starts_with(char::is_numeric) {
            return false;
        }

        name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Phase 3: Resolve all discovered types to their locations
    pub fn resolve_type_relationships(&mut self) -> Result<(), Box<dyn Error>> {
        // Take ownership of discovered types to avoid borrowing issues
        let discovered_types = std::mem::take(&mut self.discovered_types);
        for discovered_type in discovered_types.iter() {
            let location =
                self.resolve_type_location(&discovered_type.name, &discovered_type.used_in_module);

            if matches!(location, TypeLocation::Unknown) {
                eprintln!(
                    "Cannot resolve type '{}' used in module '{}'. Please use qualified paths for external types.",
                    discovered_type.name, discovered_type.used_in_module
                );
                continue;
            }

            self.resolved_types
                .insert(discovered_type.name.clone(), location.clone());
            self.add_resolved_dependency(&discovered_type.used_in_module, &location);
        }

        self.discovered_types = discovered_types;
        Ok(())
    }

    /// Resolve a type name to its location
    fn resolve_type_location(&self, type_name: &str, used_in_module: &str) -> TypeLocation {
        // Check if it's a builtin type
        if Self::PRELUDE_TYPES.contains(&type_name) {
            return TypeLocation::Builtin;
        }

        // Check if it's already resolved
        if let Some(location) = self.resolved_types.get(type_name) {
            return location.clone();
        }

        // Check if it's a framework type
        if let Some(full_path) = self.framework_types.get(type_name) {
            return TypeLocation::BloxideFramework(full_path.clone());
        }

        // Check if it might be an actor-local type
        let actor_module = used_in_module.split("::").next().unwrap_or_default();
        if !actor_module.is_empty() {
            // Check if it could be in messaging module
            let messaging_path = format!("crate::{actor_module}::messaging::{type_name}");
            if self.resolved_types.values().any(
                |loc| matches!(loc, TypeLocation::ActorCustom(path) if path == &messaging_path),
            ) {
                return TypeLocation::ActorCustom(messaging_path);
            }
        }

        TypeLocation::Unknown
    }

    /// Add a dependency based on resolved type location
    fn add_resolved_dependency(&mut self, from_module: &str, location: &TypeLocation) {
        match location {
            TypeLocation::BloxideFramework(full_path) => {
                self.add_dependency_by_path(from_module, full_path);
            }
            TypeLocation::ActorCustom(full_path) => {
                if !self.is_self_import(from_module, full_path) {
                    self.add_dependency_by_path(from_module, full_path);
                }
            }
            TypeLocation::Builtin | TypeLocation::Unknown => {}
        }
    }

    /// Phase 4: Generate imports for a module based on resolved dependencies
    pub fn generate_imports_for_module(&self, module_path: &str) -> Vec<String> {
        if let Some(module_idx) = self.graph.find_module_by_path_hierarchical(module_path) {
            self.get_imports_for_module(module_idx)
                .filter(|import| !self.is_self_import(module_path, import))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Main orchestration method: run all phases for an actor
    pub fn analyze_actor(&mut self, actor: &Actor) -> Result<(), Box<dyn Error>> {
        // Phase 1: Bootstrap framework types
        self.bootstrap_bloxide_types();

        // Phase 2: Discover all types in the actor
        self.discover_actor_types(actor)?;

        // Phase 3: Resolve type relationships
        self.resolve_type_relationships()
    }

    /// Get debug information about discovered and resolved types
    pub fn debug_type_resolution(&self) -> String {
        let mut output = String::new();
        output.push_str("=== Type Resolution Debug ===\n\n");

        output.push_str("Framework Types:\n");
        for (name, path) in &self.framework_types {
            output.push_str(&format!("  {name} -> {path}\n"));
        }

        output.push_str("\nDiscovered Types:\n");
        for discovered in &self.discovered_types {
            output.push_str(&format!(
                "  {} (in {}, context: {:?})\n",
                discovered.name, discovered.used_in_module, discovered.context
            ));
        }

        output.push_str("\nResolved Types:\n");
        for (name, location) in &self.resolved_types {
            output.push_str(&format!("  {name} -> {location:?}\n"));
        }

        output
    }
}

impl CodeGenGraph {
    /// Check if adding a dependency would result in a self-import
    fn is_self_import(&self, from_module: &str, to_path: &str) -> bool {
        // Skip external crates (they can't be self-imports)
        if !to_path.starts_with("crate::") {
            return false;
        }

        // Extract the module path from the to_path
        let to_module = if to_path.contains("::*") {
            // Glob import like "crate::actor::runtime::*"
            to_path.trim_end_matches("::*")
        } else if let Some(last_colon) = to_path.rfind("::") {
            // Type path like "crate::actor::component::SomeType" -> "crate::actor::component"
            &to_path[..last_colon]
        } else {
            // Just a simple path
            to_path
        };

        // Convert from_module to full crate path for comparison
        let from_full_path = if from_module.starts_with("crate::") {
            from_module.to_string()
        } else {
            // Convert "actor::component" to "crate::actor::component"
            format!("crate::{from_module}")
        };

        // Check if they refer to the same module
        to_module == from_full_path || to_module == from_module
    }

    /// Add a dependency between two modules/types using their string paths
    /// This is a convenience wrapper around add_dependency that handles path lookup
    pub fn add_dependency_by_path(&mut self, from_module: &str, to_path: &str) {
        // Safeguard: Check if this would be a self-import
        if self.is_self_import(from_module, to_path) {
            return; // Skip self-imports
        }

        // Get or create the "from" module node
        let from_module_idx =
            if let Some(existing) = self.graph.find_module_by_path_hierarchical(from_module) {
                existing
            } else {
                self.add_generated_module(from_module)
            };

        // Determine what type of node the "to" path represents
        let to_idx = self.get_or_create_node_by_path(to_path);

        // Add the Uses edge using the direct method
        self.graph.add_edge(from_module_idx, to_idx, Relation::Uses);
    }

    /// Get or create a node by path - useful for preparing indices for add_dependency
    pub fn get_or_create_node_by_path(&mut self, path: &str) -> NodeIndex {
        if path.ends_with("::*") {
            let module_path = path.trim_end_matches("::*");
            return self.graph.add_from_path(
                module_path,
                Node::Module(Module {
                    name: module_path.split("::").last().unwrap().to_string(),
                    path: module_path.to_string(),
                }),
            );
        }

        self.graph.add_type_from_path(path)
    }

    /// Get all imports needed for a specific module by traversing Uses edges
    pub fn get_imports_for_module(&self, module_idx: NodeIndex) -> impl Iterator<Item = String> {
        let mut imports = Vec::new();
        let module_path = self.graph.get_node_path(module_idx);

        // Find all nodes this module Uses
        let connected = self.graph.find_connected_nodes(module_idx);
        for RelatedEntry {
            index: connected_idx,
            node: _,
            relation,
        } in connected
        {
            if !matches!(relation, Relation::Uses) {
                continue;
            }
            let connected_path = self.graph.get_node_path(connected_idx);

            // Safeguard: Skip self-imports
            if self.is_self_import(&module_path, &connected_path) {
                continue;
            }

            let import_statement = self.graph.get_node_path(connected_idx);
            imports.push(Import::new(import_statement));
        }

        imports.sort();
        imports.dedup();
        imports.into_iter().map(|imp| imp.rust_import())
    }

    /// Get the full path of a node by node index (delegated to inner graph)
    pub fn get_node_path(&self, node_idx: NodeIndex) -> String {
        self.graph.get_node_path(node_idx)
    }

    /// Add a generated type to the graph and track its dependencies
    pub fn add_generated_type(&mut self, type_path: &str, dependencies: &[String]) -> NodeIndex {
        let type_idx = self.graph.add_type_from_path(type_path);

        // Add dependencies
        for dep_path in dependencies {
            let dep_idx = self.graph.add_type_from_path(dep_path);
            self.graph.add_edge(type_idx, dep_idx, Relation::Uses);
        }

        type_idx
    }

    /// Add a generated module and track its contents
    pub fn add_generated_module(&mut self, module_path: &str) -> NodeIndex {
        match self.graph.find_module_by_path_hierarchical(module_path) {
            Some(existing) => existing,
            None => self.graph.add_from_path(
                module_path,
                Node::Module(Module {
                    name: module_path.split("::").last().unwrap().to_string(),
                    path: module_path.to_string(),
                }),
            ),
        }
    }

    /// Get a visual representation of the dependency graph
    pub fn debug_dependencies(&self) -> String {
        let mut output = String::new();
        output.push_str("=== Code Generation Dependency Graph ===\n");

        for node_idx in self.graph.graph.node_indices() {
            let node = &self.graph.graph[node_idx];
            output.push_str(&format!("Node: {} ({})\n", node.name(), node.node_str()));

            for RelatedEntry { node, relation, .. } in self.graph.find_connected_nodes(node_idx) {
                output.push_str(&format!("  -> {} ({relation:?})\n", node.name()));
            }
            output.push('\n');
        }

        output
    }

    /// Extract required imports by analyzing the generated code for type usage
    pub fn extract_required_imports(&self, code: &str) -> Vec<String> {
        let mut imports = Vec::new();

        // Map of type patterns to their import paths
        let type_mappings = [
            // Core bloxide types
            ("Components", "bloxide_tokio::components::Components"),
            ("TokioMessageHandle", "bloxide_tokio::TokioMessageHandle"),
            ("TokioRuntime", "bloxide_tokio::TokioRuntime"),
            ("Runtime", "bloxide_tokio::components::Runtime"),
            ("MessageSender", "bloxide_tokio::messaging::MessageSender"),
            ("MessageSet", "bloxide_tokio::messaging::MessageSet"),
            ("Message", "bloxide_tokio::messaging::Message"),
            // State machine types
            ("StateMachine", "bloxide_tokio::state_machine::StateMachine"),
            ("State", "bloxide_tokio::state_machine::State"),
            ("StateEnum", "bloxide_tokio::state_machine::StateEnum"),
            ("Transition", "bloxide_tokio::state_machine::Transition"),
            (
                "ExtendedState",
                "bloxide_tokio::state_machine::ExtendedState",
            ),
            // Runtime types
            ("Runnable", "bloxide_tokio::components::Runnable"),
        ];

        for (type_name, import_path) in &type_mappings {
            if self.code_uses_type(code, type_name) {
                imports.push(import_path.to_string());
            }
        }

        imports
    }

    /// Check if the code uses a specific type
    pub fn code_uses_type(&self, code: &str, type_name: &str) -> bool {
        // Look for various usage patterns
        let patterns = [
            format!("impl {type_name}"), // trait implementations
            format!(": {type_name}"),    // type annotations
            format!("<{type_name}>"),    // generic parameters
            format!("{type_name}::"),    // qualified paths
            format!("{type_name}<"),     // generic type usage
            format!("as {type_name}"),   // type casts
        ];

        patterns.iter().any(|pattern| code.contains(pattern))
    }
}

#[cfg(test)]
mod tests {
    use petgraph::Direction::Incoming;

    use crate::graph::node::Type;

    use super::*;

    // Test helper functions for validating module hierarchy using petgraph algorithms

    /// Validate that the graph represents a valid module hierarchy using petgraph algorithms
    fn validate_module_hierarchy(graph: &RustGraph) -> Result<(), String> {
        // 1. Check that the graph is acyclic (required for valid hierarchy)
        if graph.is_cyclic() {
            return Err("Module hierarchy contains cycles".to_string());
        }

        // 2. Check that we can get a topological ordering
        let _topo_order = graph
            .topological_sort()
            .map_err(|_| "Cannot create topological ordering")?;

        // 3. Validate each node has at most one parent (tree property)
        for node_idx in graph.graph.node_indices() {
            let parents: Vec<_> = graph
                .graph
                .edges_directed(node_idx, Incoming)
                .filter(|e| *e.weight() == Relation::Contains)
                .collect();

            if parents.len() > 1 {
                return Err(format!(
                    "Node {:?} has multiple parents ({}), violating tree structure",
                    graph.graph[node_idx],
                    parents.len()
                ));
            }
        }

        // 4. Validate that all containment edges form a connected component
        let containment_nodes: Vec<_> = graph
            .graph
            .node_indices()
            .filter(|&idx| {
                // Include nodes that have containment edges (either incoming or outgoing)
                graph
                    .graph
                    .edges(idx)
                    .any(|e| *e.weight() == Relation::Contains)
                    || graph
                        .graph
                        .edges_directed(idx, Incoming)
                        .any(|e| *e.weight() == Relation::Contains)
            })
            .collect();

        if containment_nodes.len() > 1 {
            // Count containment edges using petgraph's edge iteration
            let containment_edge_count = graph
                .graph
                .edge_references()
                .filter(|e| *e.weight() == Relation::Contains)
                .count();

            // Count root nodes (nodes with no incoming containment edges)
            let root_count = find_hierarchy_roots(graph).len();

            // For a forest of trees, we should have exactly (n - k) edges
            // where n is number of nodes and k is number of trees (roots)
            let expected_edges = containment_nodes.len() - root_count;

            if containment_edge_count != expected_edges {
                return Err(format!(
                    "Invalid forest structure: {} nodes, {} roots, but {} containment edges (expected {})",
                    containment_nodes.len(),
                    root_count,
                    containment_edge_count,
                    expected_edges
                ));
            }
        }

        Ok(())
    }

    /// Find the root nodes (nodes with no incoming containment edges) using petgraph
    fn find_hierarchy_roots(graph: &RustGraph) -> Vec<NodeIndex> {
        graph
            .graph
            .node_indices()
            .filter(|&idx| {
                !graph
                    .graph
                    .edges_directed(idx, Incoming)
                    .any(|e| *e.weight() == Relation::Contains)
            })
            .collect()
    }

    /// Validate the complete path from root to a given node using DFS
    fn validate_node_path(
        graph: &RustGraph,
        node_idx: NodeIndex,
        expected_path: &str,
    ) -> Result<(), String> {
        let actual_path = graph.get_node_path(node_idx);
        if actual_path == expected_path {
            Ok(())
        } else {
            Err(format!(
                "Path mismatch for node {:?}: expected '{}', got '{}'",
                graph.graph[node_idx], expected_path, actual_path
            ))
        }
    }

    #[test]
    fn test_basic_path_parsing() {
        let mut graph = RustGraph::new();

        // Test adding a struct from a qualified path
        let user_idx = graph.add_type_from_path("models::user::User");
        let user_node = &graph.graph[user_idx];
        assert_eq!(user_node.name(), "User");
        assert!(matches!(user_node, Node::Type(_)));
    }

    #[test]
    fn test_module_hierarchy_creation() {
        let mut graph = RustGraph::new();

        // Add a deeply nested path
        graph.add_type_from_path("utils::db::postgres::Connection");

        // Check that all modules were created
        let utils = graph.find_by_name("utils");
        let db = graph.find_by_name("db");
        let postgres = graph.find_by_name("postgres");
        let connection = graph.find_by_name("Connection");

        assert_eq!(utils.len(), 1);
        assert_eq!(db.len(), 1);
        assert_eq!(postgres.len(), 1);
        assert_eq!(connection.len(), 1);

        // Verify the hierarchy relationships
        let utils_idx = utils[0].index;
        let db_idx = db[0].index;
        let postgres_idx = postgres[0].index;
        let connection_idx = connection[0].index;

        // Verify that each module exists and has the correct type
        assert!(matches!(graph.graph[utils_idx], Node::Module(_)));
        assert!(matches!(graph.graph[db_idx], Node::Module(_)));
        assert!(matches!(graph.graph[postgres_idx], Node::Module(_)));
        assert!(matches!(graph.graph[connection_idx], Node::Type(_)));

        // Check that we have exactly the expected number of nodes (3 modules + 1 type)
        assert_eq!(graph.graph.node_count(), 4);

        // Use petgraph-based validation for the module hierarchy structure
        validate_module_hierarchy(&graph).expect("Module hierarchy should be valid");

        // Verify there's exactly one root node using petgraph
        let roots = find_hierarchy_roots(&graph);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], utils_idx);

        // Use petgraph to verify the graph is acyclic (required for valid hierarchy)
        assert!(!graph.is_cyclic(), "Module hierarchy should be acyclic");

        // Verify topological sort works (another tree property)
        let topo_order = graph
            .topological_sort()
            .expect("Should be able to topologically sort a valid hierarchy");
        assert_eq!(topo_order.len(), 4);

        // In topological order, parents should come before children
        let utils_pos = topo_order.iter().position(|&x| x == utils_idx).unwrap();
        let db_pos = topo_order.iter().position(|&x| x == db_idx).unwrap();
        let postgres_pos = topo_order.iter().position(|&x| x == postgres_idx).unwrap();
        let connection_pos = topo_order
            .iter()
            .position(|&x| x == connection_idx)
            .unwrap();

        assert!(
            utils_pos < db_pos,
            "utils should come before db in topological order"
        );
        assert!(
            db_pos < postgres_pos,
            "db should come before postgres in topological order"
        );
        assert!(
            postgres_pos < connection_pos,
            "postgres should come before Connection in topological order"
        );

        // Validate complete paths using the helper method
        validate_node_path(&graph, utils_idx, "utils").expect("utils path should be correct");
        validate_node_path(&graph, db_idx, "utils::db").expect("db path should be correct");
        validate_node_path(&graph, postgres_idx, "utils::db::postgres")
            .expect("postgres path should be correct");
        validate_node_path(&graph, connection_idx, "utils::db::postgres::Connection")
            .expect("Connection path should be correct");
    }

    #[test]
    fn test_module_reuse() {
        let mut graph = RustGraph::new();

        // Add two items to the same module
        graph.add_type_from_path("utils::db::Connection");
        graph.add_type_from_path("utils::db::Query");

        // Should only have one utils module and one db module
        let utils_modules = graph.find_by_name("utils");
        let db_modules = graph.find_by_name("db");

        assert_eq!(utils_modules.len(), 1);
        assert_eq!(db_modules.len(), 1);

        // But should have both Connection and Query
        let connections = graph.find_by_name("Connection");
        let queries = graph.find_by_name("Query");

        assert_eq!(connections.len(), 1);
        assert_eq!(queries.len(), 1);

        // Both should be contained in the same db module
        let db_idx = db_modules[0].index;
        let db_contents = graph.find_connected_nodes(db_idx).collect::<Vec<_>>();
        assert_eq!(db_contents.len(), 2); // Should contain both Connection and Query
    }

    #[test]
    fn test_different_node_types() {
        let mut graph = RustGraph::new();

        // Add different types of nodes
        let struct_idx = graph.add_type_from_path("models::User");
        let enum_idx = graph.add_type_from_path("models::Status");
        let function_idx = graph.add_function_from_path("utils::validate");
        let trait_idx = graph.add_trait_from_path("traits::Database");

        // Verify they have the correct types
        assert!(matches!(graph.graph[struct_idx], Node::Type(_)));
        assert!(matches!(graph.graph[enum_idx], Node::Type(_)));
        assert!(matches!(graph.graph[function_idx], Node::Function(_)));
        assert!(matches!(graph.graph[trait_idx], Node::Trait(_)));

        // Verify they have correct names
        assert_eq!(graph.graph[struct_idx].name(), "User");
        assert_eq!(graph.graph[enum_idx].name(), "Status");
        assert_eq!(graph.graph[function_idx].name(), "validate");
        assert_eq!(graph.graph[trait_idx].name(), "Database");
    }

    #[test]
    fn test_path_reconstruction() {
        let mut graph = RustGraph::new();

        // Add a nested item
        let connection_idx = graph.add_type_from_path("utils::db::Connection");

        // Reconstruct the path
        let reconstructed_path = graph.get_node_path(connection_idx);
        assert_eq!(reconstructed_path, "utils::db::Connection");
    }

    #[test]
    fn test_single_component_path() {
        let mut graph = RustGraph::new();

        // Add an item with no module path
        let user_idx = graph.add_type_from_path("User");

        // Should just create the struct
        assert_eq!(graph.graph[user_idx].name(), "User");
        assert!(matches!(graph.graph[user_idx], Node::Type(_)));

        // Path reconstruction should just be the name
        let path = graph.get_node_path(user_idx);
        assert_eq!(path, "User");
    }

    #[test]
    fn test_search_by_name() {
        let mut graph = RustGraph::new();

        graph.add_type_from_path("models::User");
        graph.add_type_from_path("admin::User");

        // Should find both User structs
        let users = graph.find_by_name("User");
        assert_eq!(users.len(), 2);

        // Should find no matches for non-existent name
        let missing = graph.find_by_name("NonExistent");
        assert_eq!(missing.len(), 0);
    }

    #[test]
    fn test_search_by_partial_name() {
        let mut graph = RustGraph::new();

        graph.add_type_from_path("models::User");
        graph.add_type_from_path("models::UserService");
        graph.add_type_from_path("admin::AdminUser");

        // Search for "User" should find all three
        let user_matches = graph.find_by_partial_name("User");
        assert_eq!(user_matches.len(), 3);

        // Search for "Service" should find one
        let service_matches = graph.find_by_partial_name("Service");
        assert_eq!(service_matches.len(), 1);
    }

    #[test]
    fn test_search_by_type() {
        let mut graph = RustGraph::new();

        graph.add_type_from_path("models::User");
        graph.add_type_from_path("models::Status");
        graph.add_function_from_path("utils::validate");
        graph.add_trait_from_path("traits::Database");

        // Search by type
        let types = graph.find_by_type("Type");
        let functions = graph.find_by_type("Function");
        let traits = graph.find_by_type("Trait");
        let modules = graph.find_by_type("Module");

        assert_eq!(types.len(), 2); // User and Status are both types now
        assert_eq!(functions.len(), 1);
        assert_eq!(traits.len(), 1);
        assert!(modules.len() >= 3); // At least models, utils, traits modules
    }

    #[test]
    fn test_search_by_pattern() {
        let mut graph = RustGraph::new();

        graph.add_type_from_path("models::UserData");
        graph.add_type_from_path("admin::AdminData");
        graph.add_function_from_path("utils::validate_data");

        // Case insensitive search for "data"
        let data_matches = graph.find_by_pattern("data").collect::<Vec<_>>();
        assert_eq!(data_matches.len(), 3);

        // Case insensitive search for "DATA"
        let data_upper_matches = graph.find_by_pattern("DATA").collect::<Vec<_>>();
        assert_eq!(data_upper_matches.len(), 3);
    }

    #[test]
    fn test_connected_nodes() {
        let mut graph = RustGraph::new();

        let user_idx = graph.add_type_from_path("models::User");
        let service_idx = graph.add_type_from_path("services::UserService");

        // Add a dependency relationship
        graph.add_edge(service_idx, user_idx, Relation::Uses);

        // Find nodes connected to UserService
        let connected = graph.find_connected_nodes(service_idx);

        // Should find the User struct through Uses relationship
        let uses_relationships: Vec<_> = connected
            .filter(|entry| entry.relation() == Relation::Uses)
            .collect();
        assert_eq!(uses_relationships.len(), 1);
        assert_eq!(uses_relationships[0].node().name(), "User");
    }

    #[test]
    fn test_find_dependents() {
        let mut graph = RustGraph::new();

        let user_idx = graph.add_type_from_path("models::User");
        let service_idx = graph.add_type_from_path("services::UserService");
        let controller_idx = graph.add_type_from_path("controllers::UserController");

        // Add dependency relationships (both depend on User)
        graph.add_edge(service_idx, user_idx, Relation::Uses);
        graph.add_edge(controller_idx, user_idx, Relation::Uses);

        // Find what depends on User (excluding containment relationships)
        let dependent_names: Vec<_> = graph
            .find_dependents(user_idx)
            .filter_map(|entry| {
                if entry.relation() != Relation::Contains {
                    Some(entry.node().name())
                } else {
                    None
                }
            })
            .collect();

        // Should find both UserService and UserController
        assert_eq!(dependent_names.len(), 2);
        assert!(dependent_names.contains(&"UserService".to_string()));
        assert!(dependent_names.contains(&"UserController".to_string()));
    }

    #[test]
    fn test_find_paths() {
        let mut graph = RustGraph::new();

        let a_idx = graph.add_type_from_path("A");
        let b_idx = graph.add_type_from_path("B");
        let c_idx = graph.add_type_from_path("C");

        // Create a path A -> B -> C
        graph.add_edge(a_idx, b_idx, Relation::Uses);
        graph.add_edge(b_idx, c_idx, Relation::Uses);

        // Find paths from A to C
        let paths = graph.find_paths(a_idx, c_idx);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], vec![a_idx, b_idx, c_idx]);

        // Find paths from A to B
        let paths_ab = graph.find_paths(a_idx, b_idx);
        assert_eq!(paths_ab.len(), 1);
        assert_eq!(paths_ab[0], vec![a_idx, b_idx]);

        // Find paths from A to itself
        let paths_self = graph.find_paths(a_idx, a_idx);
        assert_eq!(paths_self.len(), 1);
        assert_eq!(paths_self[0], vec![a_idx]);
    }

    #[test]
    fn test_empty_path() {
        let mut graph = RustGraph::new();

        // Test empty path
        let idx = graph.add_from_path(
            "",
            Node::Type(Type {
                name: "Test".to_string(),
                path: "Test".to_string(),
            }),
        );
        assert_eq!(graph.graph[idx].name(), "Test");

        let path = graph.get_node_path(idx);
        assert_eq!(path, "Test");
    }

    #[test]
    fn test_complex_hierarchy() {
        let mut graph = RustGraph::new();

        // Build a complex module hierarchy
        graph.add_type_from_path("web::api::v1::users::User");
        graph.add_type_from_path("web::api::v1::posts::Post");
        graph.add_type_from_path("web::api::v2::users::User");
        graph.add_function_from_path("web::middleware::auth::verify");

        // Check that we have the right number of modules
        let v1_modules = graph.find_by_name("v1");
        let v2_modules = graph.find_by_name("v2");
        let users_modules = graph.find_by_name("users");
        let user_structs = graph.find_by_name("User");

        assert_eq!(v1_modules.len(), 1);
        assert_eq!(v2_modules.len(), 1);
        assert_eq!(users_modules.len(), 2); // v1::users and v2::users
        assert_eq!(user_structs.len(), 2); // User in both v1 and v2

        // Verify path reconstruction for complex paths
        if let Some(user_entry) = user_structs.first() {
            let path = graph.get_node_path(user_entry.index);
            assert!(path.contains("User"));
        }
    }

    #[test]
    fn test_petgraph_hierarchy_validation() {
        let mut graph = RustGraph::new();

        // Create a valid hierarchy first
        graph.add_type_from_path("utils::db::Connection");

        // Should pass validation
        assert!(validate_module_hierarchy(&graph).is_ok());
        assert!(!graph.is_cyclic(), "Module hierarchy should be acyclic");

        let roots = find_hierarchy_roots(&graph);
        assert_eq!(roots.len(), 1);

        // Should be able to topologically sort
        assert!(graph.topological_sort().is_ok());

        // Test invalid hierarchy - create a cycle by adding an edge back
        let utils = graph.find_by_name("utils")[0].index;
        let connection = graph.find_by_name("Connection")[0].index;

        // Add a cycle: Connection -> utils (invalid!)
        graph.add_edge(connection, utils, Relation::Contains);

        // Now validation should fail
        assert!(graph.is_cyclic(), "Should detect cycle");
        assert!(
            validate_module_hierarchy(&graph).is_err(),
            "Should detect invalid hierarchy"
        );

        // Test multiple roots scenario
        let mut graph2 = RustGraph::new();
        graph2.add_type_from_path("utils::db::Connection");
        graph2.add_type_from_path("models::User"); // Different root

        let roots2 = find_hierarchy_roots(&graph2);
        assert_eq!(roots2.len(), 2, "Should have two root nodes");

        // Should still be valid (forest of trees)
        assert!(
            validate_module_hierarchy(&graph2).is_ok(),
            "Forest should be valid"
        );
        assert!(!graph2.is_cyclic());
    }

    #[test]
    fn test_path_indexing_actual_failure() {
        let mut graph = RustGraph::new();

        // Create some modules first
        graph.add_type_from_path("utils::db::Connection");
        graph.add_type_from_path("utils::validation::EmailValidator");

        // Now imagine we have multiple modules with the same short name in different paths
        graph.add_type_from_path("models::db::User"); // Another "db" module!

        // THE PROBLEM: Two different "db" modules exist
        let _db_modules = graph.find_by_name("db");

        // Now, find_module_by_path_hierarchical resolves paths correctly:
        // When we look up "db" with context, we can distinguish between modules
        let utils_db_by_path = graph.find_module_by_path_hierarchical("utils::db");
        let models_db_by_path = graph.find_module_by_path_hierarchical("models::db");

        if let (Some(utils_idx), Some(models_idx)) = (utils_db_by_path, models_db_by_path) {
            let utils_path = graph.get_node_path(utils_idx);
            let models_path = graph.get_node_path(models_idx);
            assert_eq!(utils_path, "utils::db");
            assert_eq!(models_path, "models::db");
        }

        // THE IMPROVEMENT: We can find modules by their hierarchical paths
        assert!(
            graph
                .find_module_by_path_hierarchical("utils::db")
                .is_some()
        );
        assert!(
            graph
                .find_module_by_path_hierarchical("models::db")
                .is_some()
        );

        // Single names still work (returns first match in hierarchy)
        assert!(graph.find_module_by_path_hierarchical("db").is_some());

        let test_paths = ["utils", "models", "utils::db", "models::db"];
        for path in &test_paths {
            assert!(graph.find_module_by_path_hierarchical(path).is_some());
        }
    }

    #[test]
    fn test_hierarchical_path_resolution() {
        let mut graph = RustGraph::new();

        // Create the same hierarchy as before
        graph.add_type_from_path("utils::db::Connection");
        graph.add_type_from_path("utils::validation::EmailValidator");
        graph.add_type_from_path("models::db::User");

        // Verify that our graph iteration approach finds all the same nodes
        let utils_modules = graph.find_by_name("utils");
        let db_modules = graph.find_by_name("db");
        let models_modules = graph.find_by_name("models");

        assert_eq!(utils_modules.len(), 1, "Should find utils module");
        assert_eq!(db_modules.len(), 2, "Should find both db modules");
        assert_eq!(models_modules.len(), 1, "Should find models module");

        // Test hierarchical path resolution
        let utils_db = graph.find_module_by_path_hierarchical("utils::db");
        let models_db = graph.find_module_by_path_hierarchical("models::db");
        let utils_validation = graph.find_module_by_path_hierarchical("utils::validation");

        assert!(
            utils_db.is_some(),
            "Should find utils::db via graph traversal"
        );
        assert!(
            models_db.is_some(),
            "Should find models::db via graph traversal"
        );
        assert!(
            utils_validation.is_some(),
            "Should find utils::validation via graph traversal"
        );

        // Verify they're different modules
        assert_ne!(
            utils_db.unwrap(),
            models_db.unwrap(),
            "utils::db and models::db should be different modules"
        );

        // Test that paths are reconstructed correctly
        let utils_db_path = graph.get_node_path(utils_db.unwrap());
        let models_db_path = graph.get_node_path(models_db.unwrap());

        assert_eq!(utils_db_path, "utils::db");
        assert_eq!(models_db_path, "models::db");
    }

    #[test]
    fn test_unified_dependency_system() {
        let mut graph = CodeGenGraph::new();
        let module_path = "session::component";

        // Test adding external dependencies using the new unified system
        graph.add_dependency_by_path(module_path, "bloxide_tokio::components::Components");
        graph.add_dependency_by_path(module_path, "bloxide_tokio::TokioMessageHandle");
        graph.add_dependency_by_path(module_path, "crate::session::messaging::CustomArgs");

        // Get the module and check its imports
        let module_idx = graph
            .graph
            .find_module_by_path_hierarchical(module_path)
            .expect("Module should exist");
        let imports = graph.get_imports_for_module(module_idx).collect::<Vec<_>>();

        // Should generate proper import statements
        assert!(imports.iter().any(|s| s.contains("Components")));
        assert!(imports.iter().any(|s| s.contains("TokioMessageHandle")));
        assert!(imports.iter().any(|s| s.contains("CustomArgs")));
    }

    #[test]
    fn test_self_import_detection() {
        let graph = CodeGenGraph::new();

        // Test cases that should be detected as self-imports
        assert!(graph.is_self_import("session::component", "crate::session::component::SomeType"));
        assert!(graph.is_self_import("session::component", "crate::session::component::*"));
        assert!(graph.is_self_import(
            "crate::session::component",
            "crate::session::component::SomeType"
        ));

        // Test cases that should NOT be detected as self-imports
        assert!(!graph.is_self_import(
            "session::component",
            "crate::session::messaging::CustomArgs"
        ));
        assert!(!graph.is_self_import(
            "session::component",
            "bloxide_tokio::components::Components"
        ));
        assert!(!graph.is_self_import("session::component", "bloxide_tokio::TokioMessageHandle"));

        println!("✅ Self-import detection working correctly");
    }

    #[test]
    fn test_component_imports_no_self_imports() {
        let mut graph = CodeGenGraph::new();

        // Load the test actor config
        let actor_json = std::fs::read_to_string("tests/actor_config.json")
            .expect("Should be able to read test actor config");
        let actor: Actor =
            serde_json::from_str(&actor_json).expect("Should be able to parse test actor config");

        // Use the new architecture to analyze the actor
        graph
            .analyze_actor(&actor)
            .expect("Actor analysis should succeed");
        let component_module_path = "session::component";

        // Get the component module and its imports
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical(component_module_path)
            .expect("Component module should exist");
        let imports = graph
            .get_imports_for_module(component_module_idx)
            .collect::<Vec<_>>();

        assert!(imports.iter().any(|s| s.contains("CustomArgs")));
        assert!(imports.iter().any(|s| s.contains("Components")));
        assert!(imports.iter().any(|s| s.contains("TokioMessageHandle")));
        for import in &imports {
            assert!(
                !import.contains("crate::session::component"),
                "Found self-import in component module: {import}"
            );
        }

        // Verify we still have the expected external imports
    }

    #[test]
    fn test_self_import_prevention() {
        let mut graph = CodeGenGraph::new();

        // Test that self-imports are properly detected and prevented
        println!("=== Testing self-import detection ===");

        // These should be detected as self-imports
        assert!(graph.is_self_import("session::component", "crate::session::component::SomeType"));
        assert!(graph.is_self_import("session::component", "crate::session::component::*"));
        assert!(graph.is_self_import(
            "crate::session::component",
            "crate::session::component::SomeType"
        ));

        // These should NOT be detected as self-imports
        assert!(!graph.is_self_import(
            "session::component",
            "crate::session::messaging::CustomArgs"
        ));
        assert!(!graph.is_self_import(
            "session::component",
            "bloxide_tokio::components::Components"
        ));
        assert!(!graph.is_self_import("session::component", "bloxide_tokio::TokioMessageHandle"));

        // Test that self-imports are actually prevented when adding dependencies
        graph.add_dependency_by_path("session::component", "crate::session::component::SomeType"); // Should be ignored
        graph.add_dependency_by_path(
            "session::component",
            "crate::session::messaging::CustomArgs",
        ); // Should be added
        graph.add_dependency_by_path(
            "session::component",
            "bloxide_tokio::components::Components",
        ); // Should be added

        if let Some(component_idx) = graph
            .graph
            .find_module_by_path_hierarchical("session::component")
        {
            let imports = graph
                .get_imports_for_module(component_idx)
                .collect::<Vec<_>>();

            assert!(imports.iter().any(|s| s.contains("CustomArgs")));
            assert!(imports.iter().any(|s| s.contains("Components")));
            // Should not contain any self-imports
            for import in &imports {
                assert!(
                    !import.contains("crate::session::component"),
                    "Found self-import in generated imports: {import}",
                );
            }
        } else {
            panic!("Component module should exist in graph");
        }
    }

    #[test]
    fn test_add_dependency_by_path_creates_uses_relationships() {
        let mut graph = CodeGenGraph::new();

        // Add a dependency
        graph.add_dependency_by_path(
            "session::component",
            "bloxide_tokio::components::Components",
        );

        // Check that the module was created
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("session::component")
            .expect("Component module should exist");

        // Get the connections and verify the Uses relationship exists
        let connections = graph.graph.find_connected_nodes(component_module_idx);
        let uses_connections: Vec<_> = connections
            .filter(|entry| entry.relation() == Relation::Uses)
            .collect();

        assert!(
            !uses_connections.is_empty(),
            "Should have at least one Uses relationship"
        );

        // Check that it connects to the Components trait
        let components_connection = uses_connections
            .iter()
            .find(|entry| entry.node().name() == "Components");

        assert!(
            components_connection.is_some(),
            "Should have a Uses relationship to Components trait"
        );

        println!("✅ add_dependency_by_path correctly creates Uses relationships:");
        for entry in uses_connections {
            println!("  session::component --Uses--> {}", entry.node().name());
        }
    }

    #[test]
    fn test_enhanced_discovery_creates_framework_dependencies() {
        let mut graph = CodeGenGraph::new();

        // Load the test actor config
        let actor_json = std::fs::read_to_string("tests/actor_config.json")
            .expect("Should be able to read test actor config");
        let actor: Actor =
            serde_json::from_str(&actor_json).expect("Should be able to parse test actor config");

        // Run the enhanced analysis
        graph
            .analyze_actor(&actor)
            .expect("Analysis should succeed");

        // Check component module dependencies
        let component_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("session::component")
            .expect("Component module should exist");

        let component_imports = graph
            .get_imports_for_module(component_module_idx)
            .collect::<Vec<_>>();
        // Check that expected framework dependencies are present
        assert!(
            component_imports.iter().any(|s| s.contains("Components")),
            "Component should import Components trait"
        );
        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("TokioMessageHandle")),
            "Component should import TokioMessageHandle (has message handles)"
        );

        // Check that component imports its associated types
        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("SessionStates")),
            "Component should import States type (SessionStates). Found imports: {:?}",
            component_imports
        );
        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("SessionMessageSet")),
            "Component should import MessageSet type (SessionMessageSet). Found imports: {:?}",
            component_imports
        );
        assert!(
            component_imports
                .iter()
                .any(|s| s.contains("SessionExtState")),
            "Component should import ExtendedState type (SessionExtState). Found imports: {:?}",
            component_imports
        );

        // Check states module dependencies
        let states_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("session::states")
            .expect("States module should exist");

        let states_imports = graph
            .get_imports_for_module(states_module_idx)
            .collect::<Vec<_>>();

        assert!(
            states_imports.iter().any(|s| s.contains("StateMachine")),
            "States should import StateMachine trait"
        );
        assert!(
            states_imports.iter().any(|s| s.contains("State")),
            "States should import State trait"
        );

        println!("✅ Enhanced discovery methods create expected framework dependencies");
    }

    #[test]
    fn test_states_imports_actor_component() {
        let mut graph = CodeGenGraph::new();

        // Load the test actor config
        let actor_json = std::fs::read_to_string("tests/actor_config.json")
            .expect("Should be able to read test actor config");
        let actor: Actor =
            serde_json::from_str(&actor_json).expect("Should be able to parse test actor config");

        // Run the enhanced analysis
        graph
            .analyze_actor(&actor)
            .expect("Analysis should succeed");

        // Check that states module imports the actor's component
        let states_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("session::states")
            .expect("States module should exist");

        let states_imports = graph
            .get_imports_for_module(states_module_idx)
            .collect::<Vec<_>>();

        // Should import the SessionComponents from the component module
        assert!(
            states_imports
                .iter()
                .any(|s| s.contains("SessionComponents")),
            "States should import the actor's component type (SessionComponents). Found imports: {:?}",
            states_imports
        );

        // Should import the MessageSet from messaging module
        assert!(
            states_imports
                .iter()
                .any(|s| s.contains("SessionMessageSet")),
            "States should import the MessageSet type (SessionMessageSet). Found imports: {:?}",
            states_imports
        );

        // Verify the import paths are correct
        assert!(
            states_imports
                .iter()
                .any(|s| s.contains("crate::session::component::SessionComponents")),
            "Should import SessionComponents from correct path. Found imports: {:?}",
            states_imports
        );

        assert!(
            states_imports
                .iter()
                .any(|s| s.contains("crate::session::messaging::SessionMessageSet")),
            "Should import SessionMessageSet from correct path. Found imports: {:?}",
            states_imports
        );
    }

    #[test]
    fn test_messaging_imports_variant_argument_types() {
        let mut graph = CodeGenGraph::new();

        let actor_json = std::fs::read_to_string("tests/actor_config.json")
            .expect("Should be able to read test actor config");
        let actor: Actor =
            serde_json::from_str(&actor_json).expect("Should be able to parse test actor config");

        graph
            .analyze_actor(&actor)
            .expect("Analysis should succeed");

        let messaging_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("session::messaging")
            .expect("Messaging module should exist");

        let messaging_imports = graph
            .get_imports_for_module(messaging_module_idx)
            .collect::<Vec<_>>();

        // Should import framework types
        assert!(
            messaging_imports.iter().any(|s| s.contains("Message")),
            "Messaging should import Message trait. Found imports: {:?}",
            messaging_imports
        );
        assert!(
            messaging_imports.iter().any(|s| s.contains("MessageSet")),
            "Messaging should import MessageSet trait. Found imports: {:?}",
            messaging_imports
        );

        // Should import types extracted from variant arguments
        assert!(
            messaging_imports
                .iter()
                .any(|s| s.contains("StandardPayload")),
            "Messaging should import StandardPayload from variant args. Found imports: {:?}",
            messaging_imports
        );
        assert!(
            messaging_imports.iter().any(|s| s.contains("TokioRuntime")),
            "Messaging should import TokioRuntime from variant args. Found imports: {:?}",
            messaging_imports
        );
    }

    #[test]
    fn test_runtime_imports_essential_types() {
        let mut graph = CodeGenGraph::new();

        let actor_json = std::fs::read_to_string("tests/actor_config.json")
            .expect("Should be able to read test actor config");
        let actor: Actor =
            serde_json::from_str(&actor_json).expect("Should be able to parse test actor config");

        graph
            .analyze_actor(&actor)
            .expect("Analysis should succeed");

        let runtime_module_idx = graph
            .graph
            .find_module_by_path_hierarchical("session::runtime")
            .expect("Runtime module should exist");

        let runtime_imports = graph
            .get_imports_for_module(runtime_module_idx)
            .collect::<Vec<_>>();

        // Should import core runtime types
        assert!(
            runtime_imports.iter().any(|s| s.contains("Runnable")),
            "Runtime should import Runnable trait. Found imports: {:?}",
            runtime_imports
        );
        assert!(
            runtime_imports.iter().any(|s| s.contains("Blox")),
            "Runtime should import Blox type. Found imports: {:?}",
            runtime_imports
        );

        // Should import standard library types
        assert!(
            runtime_imports.iter().any(|s| s.contains("std::pin::Pin")),
            "Runtime should import Pin from std. Found imports: {:?}",
            runtime_imports
        );

        // Should import tokio macros
        assert!(
            runtime_imports.iter().any(|s| s.contains("tokio::select")),
            "Runtime should import select macro from tokio. Found imports: {:?}",
            runtime_imports
        );

        println!("✅ Runtime module correctly imports essential types");
    }
}
