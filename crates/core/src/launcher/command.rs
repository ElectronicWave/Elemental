use std::{collections::HashMap, path::PathBuf};

use crate::runtime::distribution::Distribution;

#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub runtime: Distribution,
    pub jvm_arguments: Vec<String>,
    pub main_class: String,
    pub game_arguments: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

impl LaunchCommand {
    pub fn new(
        runtime: Distribution,
        jvm_arguments: Vec<String>,
        main_class: String,
        game_arguments: Vec<String>,
    ) -> Self {
        Self {
            runtime,
            jvm_arguments,
            main_class,
            game_arguments,
            env: HashMap::new(),
            cwd: None,
        }
    }

    pub fn program(&self) -> PathBuf {
        self.runtime.executable()
    }

    pub fn arguments(&self) -> Vec<String> {
        let mut arguments = self.jvm_arguments.clone();
        arguments.push(self.main_class.clone());
        arguments.extend(self.game_arguments.clone());
        arguments
    }

    pub fn add_jvm_argument(&mut self, argument: String) {
        self.jvm_arguments.push(argument);
    }

    pub fn add_jvm_arguments(&mut self, arguments: Vec<String>) {
        self.jvm_arguments.extend(arguments);
    }

    pub fn add_game_argument(&mut self, argument: String) {
        self.game_arguments.push(argument);
    }

    pub fn add_game_arguments(&mut self, arguments: Vec<String>) {
        self.game_arguments.extend(arguments);
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
