use std::hash::RandomState;

use petgraph::{
    Directed, Direction, Graph, algo,
    graph::{EdgeIndex, NodeIndex},
};

use crate::graph::node::RustEntity;

use super::{
    RelatedEntry,
    node::{Entry, Function, Module, Node, Relation, Trait, Type},
};

#[derive(Debug, Clone)]
pub struct RustGraph {
    pub graph: Graph<Node, Relation, Directed>,
}

impl Default for RustGraph {
    fn default() -> Self {
        Self::new()
    }
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
    ) -> EdgeIndex {
        self.graph.add_edge(source, target, relation)
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
    pub fn find_by_pattern(&self, pattern: &str) -> impl Iterator<Item = Entry> {
        let pattern_lower = pattern.to_lowercase();
        self.graph.node_indices().filter_map(move |idx| {
            let node = &self.graph[idx];
            if node.name().to_lowercase().contains(&pattern_lower) {
                Some(Entry::new(idx, node))
            } else {
                None
            }
        })
    }

    // Find connected nodes using petgraph's built-in neighbors
    pub fn find_connected_nodes(&self, node_idx: NodeIndex) -> impl Iterator<Item = RelatedEntry> {
        self.graph.neighbors(node_idx).map(move |neighbor_idx| {
            // Get the edge weight by finding the edge between these nodes
            let edge_ref = self
                .graph
                .edges_connecting(node_idx, neighbor_idx)
                .next()
                .expect("Edge should exist");
            RelatedEntry::new(neighbor_idx, &self.graph[neighbor_idx], *edge_ref.weight())
        })
    }

    // Find nodes that depend on this node using petgraph's neighbors_directed
    pub fn find_dependents(&self, node_idx: NodeIndex) -> impl Iterator<Item = RelatedEntry> {
        self.graph
            .neighbors_directed(node_idx, Direction::Incoming)
            .map(move |dependent_idx| {
                // Get the edge weight by finding the edge between these nodes
                let edge_ref = self
                    .graph
                    .edges_connecting(dependent_idx, node_idx)
                    .next()
                    .expect("Edge should exist");
                RelatedEntry::new(
                    dependent_idx,
                    &self.graph[dependent_idx],
                    *edge_ref.weight(),
                )
            })
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
        let mut current_path = String::with_capacity(module_segments.len() * 3);

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
    pub fn find_module_by_path_hierarchical(&self, path: &str) -> Option<NodeIndex> {
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
            current_module = self.graph.neighbors(current_module).find(|child_idx| {
                // Must be a module with Contains relation and matching name
                matches!(self.graph[*child_idx], Node::Module(_))
                    && self.graph[*child_idx].name() == segment
                    && self
                        .graph
                        .edges_connecting(current_module, *child_idx)
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
        // Use the stored full path from the node instead of reconstructing via hierarchy
        if let Some(node) = self.graph.node_weight(node_idx) {
            node.full_path()
        } else {
            String::new()
        }
    }
}
