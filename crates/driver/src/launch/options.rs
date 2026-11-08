use std::path::PathBuf;

use elemental_core::runtime::RuntimeValidationMode;

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub runtime_major_version: Option<usize>,
    pub runtime_executable_path: Option<PathBuf>,
    pub runtime_validation: RuntimeValidationMode,
}

impl RuntimeOptions {
    pub fn new() -> Self {
        Self {
            runtime_major_version: None,
            runtime_executable_path: None,
            runtime_validation: RuntimeValidationMode::Strict,
        }
    }

    pub fn set_runtime_major_version(mut self, runtime_major_version: usize) -> Self {
        self.runtime_major_version = Some(runtime_major_version);
        self
    }

    pub fn set_runtime_executable_path(mut self, runtime_executable_path: PathBuf) -> Self {
        self.runtime_executable_path = Some(runtime_executable_path);
        self
    }

    pub fn set_runtime_validation(mut self, runtime_validation: RuntimeValidationMode) -> Self {
        self.runtime_validation = runtime_validation;
        self
    }
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self::new()
    }
}
