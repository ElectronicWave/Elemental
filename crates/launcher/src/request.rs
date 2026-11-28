pub use elemental_driver::launch::RuntimeOptions;

use crate::addon::AddonSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResolution {
    pub width: String,
    pub height: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickPlayOptions {
    pub path: Option<String>,
    pub multiplayer: Option<String>,
    pub singleplayer: Option<String>,
    pub realms: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    pub launcher_name: Option<String>,
    pub launcher_version: Option<String>,
    pub client_id: Option<String>,
    pub resolution: Option<LaunchResolution>,
    pub quick_play: Option<QuickPlayOptions>,
    pub extra_jvm_arguments: Vec<String>,
    pub extra_game_arguments: Vec<String>,
    /// Ephemeral addons applied only for this launch (prepared, overlaid in memory,
    /// and hooked via `augment_launch` without persisting to `.elemental/addons.json`).
    pub addons: Vec<AddonSpec>,
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

impl LaunchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_extra_jvm_arguments(mut self, extra_jvm_arguments: Vec<String>) -> Self {
        self.extra_jvm_arguments = extra_jvm_arguments;
        self
    }

    pub fn set_extra_game_arguments(mut self, extra_game_arguments: Vec<String>) -> Self {
        self.extra_game_arguments = extra_game_arguments;
        self
    }

    /// Attaches an ephemeral addon to be prepared, overlaid in memory, and hooked
    /// for this launch without modifying the instance's persistent addon state.
    pub fn addon(mut self, spec: AddonSpec) -> Self {
        self.addons.push(spec);
        self
    }
}
