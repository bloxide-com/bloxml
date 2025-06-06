use crate::blox::actor::Actor;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
};

use super::{
    ext_state_gen, generate_component, generate_message_set, generate_state_enum_impl,
    runtime_gen::generate_runtime, state_gen,
};

const MSG_MOD: &str = "messaging.rs";
const EXT_STATE_MOD: &str = "ext_state.rs";
const COMPONENT_MOD: &str = "component.rs";
const RUNTIME_MOD: &str = "runtime.rs";

const MODS: [&str; 4] = [MSG_MOD, EXT_STATE_MOD, COMPONENT_MOD, RUNTIME_MOD];

fn create_module_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|e| format!("Error creating directory {}: {e}", path.display()))
}

fn create_states_module(path: &Path, actor: &Actor) -> Result<(), Box<dyn Error>> {
    create_module_dir(path)?;
    create_state_files(path, actor)?;

    let states_mod_rs = actor
        .component
        .states
        .states
        .iter()
        .map(|state| format!("pub mod {};", state.ident.to_lowercase()))
        .fold(String::new(), |acc, s| format!("{acc}{s}\n"));

    let mod_rs = path.join("mod.rs");
    let mut mod_rs =
        File::create(&mod_rs).map_err(|e| format!("Error creating states/mod.rs: {}", e))?;

    mod_rs
        .write_all(states_mod_rs.as_bytes())
        .map_err(|e| format!("Error writing states/mod.rs: {e}"))?;

    mod_rs
        .write_all(generate_state_enum_impl(actor)?.as_bytes())
        .map_err(|e| format!("Error writing states/mod.rs: {e}").into())
}

fn create_state_files(path: &Path, actor: &Actor) -> Result<(), Box<dyn Error>> {
    let states = &actor.component.states;
    let state_files = states
        .states
        .iter()
        .map(|state| state.ident.to_lowercase())
        .map(|mod_file| path.join(format!("{mod_file}.rs")))
        .map(File::create)
        .collect::<Result<Vec<File>, _>>()?;

    states
        .states
        .iter()
        .zip(state_files)
        .try_for_each(|(state, mut file)| {
            let impl_content = state_gen::generate_inner_states(actor, state)?;
            file.write_all(impl_content.as_bytes())
                .map_err(|e| format!("Error writing state impl: {e}").into())
        })
}

fn create_module_files(mod_path: &Path, mods: &[&str]) -> Result<(), Box<dyn Error>> {
    mods.iter()
        .map(|mod_file| mod_path.join(mod_file))
        .map(File::create)
        .try_for_each(|res| {
            res.map(|_| ())
                .map_err(|e| format!("Error creating file: {e}").into())
        })
}

fn create_root_mod_rs(mod_path: &Path, mods: &[&str]) -> Result<(), Box<dyn Error>> {
    let modules: Vec<String> = mods
        .iter()
        .map(|mod_file| mod_file.split('.').next().unwrap().to_string())
        .collect();

    let mod_rs_content = modules
        .iter()
        .map(|mod_name| format!("pub mod {};", mod_name))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(mod_path.join("mod.rs"), mod_rs_content)
        .map_err(|e| format!("Error creating mod.rs file: {e}").into())
}

pub fn create_module(actor: &Actor) -> Result<(), Box<dyn Error>> {
    actor.component.states.validate()?;

    let mod_path = actor.create_mod_path();
    create_module_dir(&mod_path)?;
    create_module_files(&mod_path, &MODS)?;

    let states_path = actor.create_states_path();
    create_states_module(&states_path, actor)?;

    if let Some(message_set) = &actor.component.message_set {
        let message_module_content = generate_message_set(message_set)?;
        fs::write(mod_path.join("messaging.rs"), message_module_content)?;
    }

    // Generate component.rs
    let component_content = generate_component(actor);
    fs::write(mod_path.join(COMPONENT_MOD), component_content)?;

    // Generate placeholder files for ext_state.rs and runtime.rs
    let placeholder_ext_state = format!(
        r#"//! # {ident} Extended State
//! 
//! Extended state for the {ident} component.
//! This file defines the extended state data structure that persists across state transitions.

/// Extended state for the {ident} component
{ext_state}
"#,
        ident = actor.ident,
        ext_state = ext_state_gen::generate_ext_state(&actor.component.ext_state),
    );
    fs::write(mod_path.join(EXT_STATE_MOD), placeholder_ext_state)?;

    let runtime_content = generate_runtime(actor)?;
    fs::write(mod_path.join(RUNTIME_MOD), runtime_content)?;

    create_root_mod_rs(&mod_path, &MODS)
}

#[cfg(test)]
mod tests {

    use super::create_module;
    use crate::tests::create_test_actor;
    use std::path::Path;

    const TEST_PATH: &str = "tests/output";

    #[test]
    fn test_create_module_dir() {
        let path = Path::new(TEST_PATH);
        let test_actor = create_test_actor();
        create_module(&test_actor).expect("Failed to create module");
        assert!(path.join(test_actor.ident.to_lowercase()).exists());
    }
}
