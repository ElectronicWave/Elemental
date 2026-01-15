use std::path::PathBuf;

pub use super::super::provider::RuntimeProvider;
#[derive(Default)]
pub struct RegistryProvider;

impl RuntimeProvider for RegistryProvider {
    fn list(&self) -> Vec<PathBuf> {
        // Implementation here
        vec![]
    }

    fn name(&self) -> &'static str {
        return "Registry";
    }
}
