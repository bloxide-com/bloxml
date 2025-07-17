mod node;

use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Direction, Graph, Incoming, algo};
use std::collections::hash_map::RandomState;

pub use node::*;

pub struct RustGraph {
    pub graph: Graph<Node, Relation, Directed>,
}

impl RustGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    // Re-export petgraph graph analysis algorithms

    /// Check if the graph is cyclic
    pub fn is_cyclic(&self) -> bool {
        algo::is_cyclic_directed(&self.graph)
    }

    /// Get strongly connected components
    pub fn strongly_connected_components(&self) -> Vec<Vec<NodeIndex>> {
        algo::tarjan_scc(&self.graph)
    }

    /// Perform topological sort (if acyclic)
    pub fn topological_sort(&self) -> Result<Vec<NodeIndex>, algo::Cycle<NodeIndex>> {
        algo::toposort(&self.graph, None)
    }

    pub fn add_node(&mut self, node: Node) -> NodeIndex {
        self.graph.add_node(node)
    }

    pub fn add_edge(
        &mut self,
        source: NodeIndex,
        target: NodeIndex,
        relation: Relation,
    ) -> Option<EdgeIndex> {
        Some(self.graph.add_edge(source, target, relation))
    }

    // Find nodes by exact name match (now using graph iteration - simpler!)
    pub fn find_by_name(&self, name: &str) -> Vec<Entry> {
        self.graph
            .node_indices()
            .filter_map(|idx| {
                let node = &self.graph[idx];
                if node.name() == name {
                    Some(Entry::new(idx, node))
                } else {
                    None
                }
            })
            .collect()
    }

    // Find nodes by partial name match (now using graph iteration - simpler!)
    pub fn find_by_partial_name(&self, partial_name: &str) -> Vec<Entry> {
        self.graph
            .node_indices()
            .filter_map(|idx| {
                let node = &self.graph[idx];
                if node.name().contains(partial_name) {
                    Some(Entry::new(idx, node))
                } else {
                    None
                }
            })
            .collect()
    }

    // Find nodes by type
    pub fn find_by_type(&self, node_type: &str) -> Vec<Entry> {
        self.graph
            .node_indices()
            .filter_map(|idx| {
                let node = &self.graph[idx];
                if node.node_str() == node_type {
                    Some(Entry::new(idx, node))
                } else {
                    None
                }
            })
            .collect()
    }

    // Find nodes by name pattern (case insensitive, now using graph iteration - simpler!)
    pub fn find_by_pattern(&self, pattern: &str) -> Vec<Entry> {
        let pattern_lower = pattern.to_lowercase();
        self.graph
            .node_indices()
            .filter_map(|idx| {
                let node = &self.graph[idx];
                if node.name().to_lowercase().contains(&pattern_lower) {
                    Some(Entry::new(idx, node))
                } else {
                    None
                }
            })
            .collect()
    }

    // Find connected nodes using petgraph's built-in neighbors
    pub fn find_connected_nodes(&self, node_idx: NodeIndex) -> Vec<(NodeIndex, &Node, Relation)> {
        self.graph
            .neighbors(node_idx)
            .map(|neighbor_idx| {
                // Get the edge weight by finding the edge between these nodes
                let edge_ref = self
                    .graph
                    .edges_connecting(node_idx, neighbor_idx)
                    .next()
                    .expect("Edge should exist");
                (neighbor_idx, &self.graph[neighbor_idx], *edge_ref.weight())
            })
            .collect()
    }

    // Find nodes that depend on this node using petgraph's neighbors_directed
    pub fn find_dependents(&self, node_idx: NodeIndex) -> Vec<(NodeIndex, &Node, Relation)> {
        self.graph
            .neighbors_directed(node_idx, Direction::Incoming)
            .map(|dependent_idx| {
                // Get the edge weight by finding the edge between these nodes
                let edge_ref = self
                    .graph
                    .edges_connecting(dependent_idx, node_idx)
                    .next()
                    .expect("Edge should exist");
                (
                    dependent_idx,
                    &self.graph[dependent_idx],
                    *edge_ref.weight(),
                )
            })
            .collect()
    }

    // Find all simple paths between two nodes using petgraph's algorithm
    pub fn find_paths(&self, from: NodeIndex, to: NodeIndex) -> Vec<Vec<NodeIndex>> {
        if from == to {
            // Special case: petgraph doesn't include trivial self-paths
            return vec![vec![from]];
        }

        algo::all_simple_paths::<Vec<_>, _, RandomState>(&self.graph, from, to, 0, None).collect()
    }

    pub fn add_from_path(&mut self, path: &str, final_type: Node) -> NodeIndex {
        if path.is_empty() {
            return self.add_node(final_type);
        }

        let segments: Vec<&str> = path.split("::").collect();

        if segments.len() == 1 {
            // No modules, just the final type
            return self.add_node(final_type);
        }

        // Split into module segments and final type name
        let module_segments = &segments[..segments.len() - 1];

        let mut current_parent: Option<NodeIndex> = None;
        let mut current_path = String::new();

        // Create or find each module in the hierarchy
        for (i, segment) in module_segments.iter().enumerate() {
            if i > 0 {
                current_path.push_str("::");
            }
            current_path.push_str(segment);

            // Check if this exact module path already exists
            let module_idx =
                if let Some(existing) = self.find_module_by_path_hierarchical(&current_path) {
                    existing
                } else {
                    // Create new module with just the segment name
                    let module_node = Node::Module(Module {
                        name: segment.to_string(),
                        path: current_path.clone(),
                    });
                    let new_idx = self.add_node(module_node);

                    // No longer need dual indexing - graph traversal handles path resolution

                    // Connect to parent if exists
                    if let Some(parent_idx) = current_parent {
                        self.add_edge(parent_idx, new_idx, Relation::Contains);
                    }

                    new_idx
                };

            current_parent = Some(module_idx);
        }

        // Add the final type
        let final_idx = self.add_node(final_type);

        // Connect to the last module if exists
        if let Some(parent_idx) = current_parent {
            self.add_edge(parent_idx, final_idx, Relation::Contains);
        }

        final_idx
    }

    // Find a module by its full path using simple step-by-step traversal (MUCH BETTER!)
    fn find_module_by_path_hierarchical(&self, path: &str) -> Option<NodeIndex> {
        let segments: Vec<&str> = path.split("::").collect();
        if segments.is_empty() {
            return None;
        }

        // Start by finding the root module using our existing name index
        let root_segment = segments[0];
        let root_candidates = self.find_by_name(root_segment);

        // Find the root module (not other node types)
        let mut current_module = root_candidates
            .into_iter()
            .find(|entry| matches!(self.graph[entry.index], Node::Module(_)))?
            .index;

        // Traverse the path step by step through the remaining segments
        for &segment in &segments[1..] {
            // Look for a child module with the matching name
            current_module = self.graph.neighbors(current_module).find(|&child_idx| {
                // Must be a module with Contains relation and matching name
                matches!(self.graph[child_idx], Node::Module(_))
                    && self.graph[child_idx].name() == segment
                    && self
                        .graph
                        .edges_connecting(current_module, child_idx)
                        .any(|edge| *edge.weight() == Relation::Contains)
            })?;
        }

        Some(current_module)
    }

    pub fn add_type_from_path(&mut self, path: &str) -> NodeIndex {
        let name = path.split("::").last().unwrap().to_string();
        self.add_from_path(
            path,
            Node::Type(Type {
                name: name.clone(),
                path: path.to_string(),
            }),
        )
    }

    pub fn add_function_from_path(&mut self, path: &str) -> NodeIndex {
        let name = path.split("::").last().unwrap().to_string();
        self.add_from_path(
            path,
            Node::Function(Function {
                name: name.clone(),
                path: path.to_string(),
            }),
        )
    }

    pub fn add_trait_from_path(&mut self, path: &str) -> NodeIndex {
        let name = path.split("::").last().unwrap().to_string();
        self.add_from_path(
            path,
            Node::Trait(Trait {
                name: name.clone(),
                path: path.to_string(),
            }),
        )
    }

    pub fn get_node_path(&self, node_idx: NodeIndex) -> String {
        let mut path = Vec::new();
        let mut current_idx = node_idx;
        while let Some(node) = self.graph.node_weight(current_idx) {
            path.push(node.name().to_string());
            if let Some(parent_idx) = self
                .graph
                .edges_directed(current_idx, Incoming)
                .next()
                .map(|edge| edge.source())
            {
                current_idx = parent_idx;
            } else {
                break;
            }
        }
        path.into_iter().rev().collect::<Vec<_>>().join("::")
    }
}

#[cfg(test)]
mod tests {
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
                let has_containment = graph
                    .graph
                    .edges(idx)
                    .any(|e| *e.weight() == Relation::Contains)
                    || graph
                        .graph
                        .edges_directed(idx, Incoming)
                        .any(|e| *e.weight() == Relation::Contains);
                has_containment
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
        let db_contents = graph.find_connected_nodes(db_idx);
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
        let data_matches = graph.find_by_pattern("data");
        assert_eq!(data_matches.len(), 3);

        // Case insensitive search for "DATA"
        let data_upper_matches = graph.find_by_pattern("DATA");
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
            .iter()
            .filter(|(_, _, rel)| *rel == Relation::Uses)
            .collect();
        assert_eq!(uses_relationships.len(), 1);
        assert_eq!(uses_relationships[0].1.name(), "User");
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
        let dependents: Vec<_> = graph
            .find_dependents(user_idx)
            .into_iter()
            .filter(|(_, _, rel)| *rel != Relation::Contains)
            .collect();

        // Should find both UserService and UserController
        assert_eq!(dependents.len(), 2);
        let dependent_names: Vec<_> = dependents.iter().map(|(_, node, _)| node.name()).collect();
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
        assert!(!graph.is_cyclic());

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
    fn test_auto_module_connection() {
        let mut graph = RustGraph::new();

        // Create a module hierarchy
        graph.add_type_from_path("utils::db::Connection");
        graph.add_type_from_path("utils::validation::EmailValidator");

        // Find the utils module
        let utils_modules = graph.find_by_name("utils");
        assert_eq!(
            utils_modules.len(),
            1,
            "Should find exactly one utils module"
        );
        let utils_idx = utils_modules[0].index;

        // Get all nodes connected to utils with Contains relationship
        let connected_nodes = graph.find_connected_nodes(utils_idx);
        let children: Vec<_> = connected_nodes
            .iter()
            .filter(|(_, _, relation)| *relation == Relation::Contains)
            .collect();

        // Verify utils has exactly 2 children
        assert_eq!(
            children.len(),
            2,
            "utils module should have exactly 2 children"
        );

        // Get the names of the children
        let child_names: Vec<String> = children.iter().map(|(_, node, _)| node.name()).collect();

        // Verify the children are "db" and "validation"
        assert!(
            child_names.contains(&"db".to_string()),
            "utils should contain db module"
        );
        assert!(
            child_names.contains(&"validation".to_string()),
            "utils should contain validation module"
        );

        // Verify both children are modules
        for (_, node, _) in &children {
            assert!(
                matches!(node, Node::Module(_)),
                "Children should be Module nodes"
            );
        }
    }
}
