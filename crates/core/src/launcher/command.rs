use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

impl LaunchCommand {
    pub fn new(program: PathBuf, args: Vec<String>) -> Self {
        Self {
            program,
            args,
            env: HashMap::new(),
            cwd: None,
        }
    }

    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.insert(key, value);
        self
    }
}
