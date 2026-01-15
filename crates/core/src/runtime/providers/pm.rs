use std::path::PathBuf;

use super::super::provider::RuntimeProvider;

pub struct PackageManagerProvider;

impl RuntimeProvider for PackageManagerProvider {
    fn list(&self) -> Vec<PathBuf> {
        // Implementation here
        vec![]
    }

    fn name(&self) -> &'static str {
        return "PackageManager";
    }
}
