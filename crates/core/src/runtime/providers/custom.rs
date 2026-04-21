use std::{env::consts::EXE_SUFFIX, path::PathBuf};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;

use super::super::provider::RuntimeProvider;

pub struct CustomProvider {
    paths: Vec<PathBuf>,
}

#[async_trait]
impl RuntimeProvider for CustomProvider {
    async fn list(&self) -> Vec<PathBuf> {
        self.paths.clone()
    }

    fn name(&self) -> &'static str {
        "Custom"
    }
}

impl CustomProvider {
    pub fn new(paths: Vec<PathBuf>) -> Result<Self> {
        let mut normalized = Vec::with_capacity(paths.len());
        for path in paths {
            normalized.push(validate_runtime_path(path)?);
        }

        Ok(Self { paths: normalized })
    }
}

pub fn new_custom_provider(paths: Vec<PathBuf>) -> Result<CustomProvider> {
    CustomProvider::new(paths)
}

fn validate_runtime_path(path: PathBuf) -> Result<PathBuf> {
    let java_executable = path.join("bin").join(format!("java{EXE_SUFFIX}"));
    if !path.is_dir() {
        bail!("custom runtime path is not a directory: {}", path.display());
    }
    if !java_executable.exists() {
        bail!(
            "custom runtime path does not contain a java executable: {}",
            path.display()
        );
    }

    path.canonicalize().with_context(|| {
        format!(
            "canonicalize custom runtime path failed: {}",
            path.display()
        )
    })
}
