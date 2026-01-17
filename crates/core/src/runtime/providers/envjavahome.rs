use std::env::var;
use std::path::PathBuf;

use async_trait::async_trait;

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct EnvJavaHomeProvider;

#[async_trait]
impl RuntimeProvider for EnvJavaHomeProvider {
    async fn list(&self) -> Vec<PathBuf> {
        //TODO: Need to specifically validate the JAVA_HOME path
        var("JAVA_HOME")
            .ok()
            .map(PathBuf::from)
            .into_iter()
            .collect()
    }

    fn name(&self) -> &'static str {
        "JavaHome"
    }
}
