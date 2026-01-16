use std::env::var;
use std::path::PathBuf;

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct EnvJavaHomeProvider;

impl RuntimeProvider for EnvJavaHomeProvider {
    fn list(&self) -> Vec<PathBuf> {
        //TODO: Need to specifically validate the JAVA_HOME path
        var("JAVA_HOME")
            .ok()
            .map(PathBuf::from)
            .into_iter()
            .collect()
    }

    fn name(&self) -> &'static str {
        "EnvJavaHome"
    }
}
