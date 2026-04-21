use anyhow::Result;

use crate::launch_arguments::parse_argument_string;

#[derive(Clone)]
pub struct LaunchResolution {
    pub width: String,
    pub height: String,
}

#[derive(Clone)]
pub struct QuickPlayOptions {
    pub path: Option<String>,
    pub multiplayer: Option<String>,
    pub singleplayer: Option<String>,
    pub realms: Option<String>,
}

#[derive(Clone)]
pub struct VanillaLaunchConfig {
    pub runtime_major_version: Option<usize>,
    pub launcher_name: Option<String>,
    pub launcher_version: Option<String>,
    pub client_id: Option<String>,
    pub resolution: Option<LaunchResolution>,
    pub quick_play: Option<QuickPlayOptions>,
    pub extra_jvm_arguments: Vec<String>,
    pub extra_game_arguments: Vec<String>,
}

impl LaunchResolution {
    pub fn new(width: String, height: String) -> Self {
        Self { width, height }
    }
}

impl QuickPlayOptions {
    pub fn new(
        path: Option<String>,
        multiplayer: Option<String>,
        singleplayer: Option<String>,
        realms: Option<String>,
    ) -> Self {
        Self {
            path,
            multiplayer,
            singleplayer,
            realms,
        }
    }
}

impl VanillaLaunchConfig {
    pub fn new() -> Self {
        Self {
            runtime_major_version: None,
            launcher_name: None,
            launcher_version: None,
            client_id: None,
            resolution: None,
            quick_play: None,
            extra_jvm_arguments: Vec::new(),
            extra_game_arguments: Vec::new(),
        }
    }

    pub fn set_extra_jvm_arguments(mut self, extra_jvm_arguments: Vec<String>) -> Self {
        self.extra_jvm_arguments = extra_jvm_arguments;
        self
    }

    pub fn set_extra_game_arguments(mut self, extra_game_arguments: Vec<String>) -> Self {
        self.extra_game_arguments = extra_game_arguments;
        self
    }

    pub fn try_set_extra_jvm_argument_string(
        mut self,
        extra_jvm_argument_string: String,
    ) -> Result<Self> {
        self.extra_jvm_arguments = parse_argument_string(extra_jvm_argument_string.as_str())?;
        Ok(self)
    }

    pub fn try_set_extra_game_argument_string(
        mut self,
        extra_game_argument_string: String,
    ) -> Result<Self> {
        self.extra_game_arguments = parse_argument_string(extra_game_argument_string.as_str())?;
        Ok(self)
    }
}

impl Default for VanillaLaunchConfig {
    fn default() -> Self {
        Self::new()
    }
}
