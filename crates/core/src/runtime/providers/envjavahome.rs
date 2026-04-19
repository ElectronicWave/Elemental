use std::env::consts::EXE_SUFFIX;
use std::env::var;
use std::path::PathBuf;

use async_trait::async_trait;

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct EnvJavaHomeProvider;

#[async_trait]
impl RuntimeProvider for EnvJavaHomeProvider {
    async fn list(&self) -> Vec<PathBuf> {
        let Some(java_home) = var("JAVA_HOME").ok().map(PathBuf::from) else {
            return Vec::new();
        };

        let java_executable = java_home.join("bin").join(format!("java{EXE_SUFFIX}"));
        if java_home.is_dir() && java_executable.exists() {
            vec![java_home]
        } else {
            Vec::new()
        }
    }

    fn name(&self) -> &'static str {
        "JavaHome"
    }
}
