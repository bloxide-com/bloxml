use crate::blox::ext_state::ExtState;
use crate::graph::CodeGenGraph;

use super::ToRust;

pub fn generate_ext_state(ext_state: &ExtState, graph: &mut CodeGenGraph) -> String {
    ext_state.to_rust(graph)
}
