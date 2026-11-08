use serde::{Deserialize, Serialize};

mod apply;
mod offline;

use super::classpath::classpath_separator;

#[derive(Debug, Deserialize, Serialize)]
pub struct LauncherVariables {
    pub auth_player_name: String,
    pub version_name: String,
    pub game_directory: String,
    pub assets_root: String,
    pub assets_index_name: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    // Legacy 1.7-era launchers still expect this placeholder to be a JSON object string.
    pub user_properties: String,
    pub clientid: String,
    pub auth_xuid: String,
    pub user_type: UserType,
    pub version_type: String,
    pub resolution_width: String,
    pub resolution_height: String,
    #[serde(rename = "quickPlayPath")]
    pub quick_play_path: Option<String>,
    #[serde(rename = "quickPlaySingleplayer")]
    pub quick_play_singleplayer: Option<String>,
    #[serde(rename = "quickPlayMultiplayer")]
    pub quick_play_multiplayer: Option<String>,
    #[serde(rename = "quickPlayRealms")]
    pub quick_play_realms: Option<String>,
    pub natives_directory: String,
    pub launcher_name: String,
    pub launcher_version: String,
    pub library_directory: String,
    pub classpath_separator: String,
    pub classpath: String,
}

impl Default for LauncherVariables {
    fn default() -> Self {
        Self {
            auth_player_name: Default::default(),
            version_name: Default::default(),
            game_directory: Default::default(),
            assets_root: Default::default(),
            assets_index_name: Default::default(),
            auth_uuid: Default::default(),
            auth_access_token: Default::default(),
            user_properties: "{}".to_owned(),
            clientid: Default::default(),
            auth_xuid: Default::default(),
            user_type: UserType::Msa,
            version_type: Default::default(),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: Default::default(),
            quick_play_singleplayer: Default::default(),
            quick_play_multiplayer: Default::default(),
            quick_play_realms: Default::default(),
            natives_directory: Default::default(),
            launcher_name: "Elemental".to_owned(),
            launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
            library_directory: Default::default(),
            classpath_separator: classpath_separator().to_owned(),
            classpath: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserType {
    Legacy,
    Msa,
    Mojang,
}

impl UserType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Msa => "msa",
            Self::Mojang => "mojang",
        }
    }
}

impl LauncherVariables {
    pub(super) fn value_for(&self, key: &str) -> Option<&str> {
        match key {
            "auth_player_name" => Some(self.auth_player_name.as_str()),
            "version_name" => Some(self.version_name.as_str()),
            "game_directory" => Some(self.game_directory.as_str()),
            "assets_root" => Some(self.assets_root.as_str()),
            "assets_index_name" => Some(self.assets_index_name.as_str()),
            "auth_uuid" => Some(self.auth_uuid.as_str()),
            "auth_access_token" => Some(self.auth_access_token.as_str()),
            "user_properties" => Some(self.user_properties.as_str()),
            "clientid" => Some(self.clientid.as_str()),
            "auth_xuid" => Some(self.auth_xuid.as_str()),
            "user_type" => Some(self.user_type.as_str()),
            "version_type" => Some(self.version_type.as_str()),
            "resolution_width" => Some(self.resolution_width.as_str()),
            "resolution_height" => Some(self.resolution_height.as_str()),
            "quickPlayPath" => self.quick_play_path.as_deref(),
            "quickPlaySingleplayer" => self.quick_play_singleplayer.as_deref(),
            "quickPlayMultiplayer" => self.quick_play_multiplayer.as_deref(),
            "quickPlayRealms" => self.quick_play_realms.as_deref(),
            "natives_directory" => Some(self.natives_directory.as_str()),
            "launcher_name" => Some(self.launcher_name.as_str()),
            "launcher_version" => Some(self.launcher_version.as_str()),
            "library_directory" => Some(self.library_directory.as_str()),
            "classpath_separator" => Some(self.classpath_separator.as_str()),
            "classpath" => Some(self.classpath.as_str()),
            _ => None,
        }
    }
}
