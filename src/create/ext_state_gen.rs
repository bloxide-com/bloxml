use crate::blox::ext_state::ExtState;

use super::ToRust;

pub fn generate_ext_state(ext_state: &ExtState) -> String {
    ext_state.to_rust()
}
