use std::path::PathBuf;

use async_trait::async_trait;

use super::super::provider::RuntimeProvider;
use crate::runtime::discovery::is_runtime_home;

#[derive(Default)]
pub struct EnvJavaHomeProvider;

#[async_trait]
impl RuntimeProvider for EnvJavaHomeProvider {
    async fn list(&self) -> Vec<PathBuf> {
        let mut homes = Vec::new();
        for variable_name in ["JAVA_HOME", "JRE_HOME"] {
            let Some(java_home) = std::env::var_os(variable_name).map(PathBuf::from) else {
                continue;
            };
            if is_runtime_home(&java_home).await {
                homes.push(java_home);
            }
        }
        homes.sort();
        homes.dedup();
        homes
    }

    fn name(&self) -> &'static str {
        "JavaHome"
    }
}
