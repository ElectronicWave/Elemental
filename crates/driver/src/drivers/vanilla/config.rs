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
        }
    }
}
