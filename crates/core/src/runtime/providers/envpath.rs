use std::path::PathBuf;

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct EnvPathProvider;
impl RuntimeProvider for EnvPathProvider {
    fn list(&self) -> Vec<PathBuf> {
        // Implementation here
        vec![]
    }

    fn name(&self) -> &'static str {
        return "Environment";
    }
}
