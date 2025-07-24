use crate::blox::actor::Actor;
use crate::create::ActorGenerator;
use std::error::Error;

/// Creates the actor module
pub fn create_module(actor: Actor) -> Result<(), Box<dyn Error>> {
    let mut generator = ActorGenerator::new(actor)?;
    generator.generate_all_files()
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
        let ident = test_actor.ident.to_lowercase();
        create_module(test_actor).expect("Failed to create module");
        assert!(path.join(ident).exists());
    }
}
