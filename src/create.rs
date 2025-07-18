mod component_gen;
mod ext_state_gen;
mod file_gen;
mod message_gen;
mod runtime_gen;
mod state_gen;

pub use component_gen::*;
pub use file_gen::*;
pub use message_gen::*;
pub use state_gen::*;

use crate::graph::CodeGenGraph;

pub trait ToRust {
    fn to_rust(&self, graph: &mut CodeGenGraph) -> String;
}
