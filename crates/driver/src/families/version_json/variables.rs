mod apply;
mod offline;

use super::classpath::classpath_separator;

#[derive(Clone, Debug)]
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
    pub quick_play_path: Option<String>,
    pub quick_play_singleplayer: Option<String>,
    pub quick_play_multiplayer: Option<String>,
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

#[derive(Clone, Debug)]
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
    pub fn get(&self, key: &str) -> Option<&str> {
        Some(match key {
            "auth_player_name" => &self.auth_player_name,
            "version_name" => &self.version_name,
            "game_directory" => &self.game_directory,
            "assets_root" => &self.assets_root,
            "assets_index_name" => &self.assets_index_name,
            "auth_uuid" => &self.auth_uuid,
            "auth_access_token" => &self.auth_access_token,
            "user_properties" => &self.user_properties,
            "clientid" => &self.clientid,
            "auth_xuid" => &self.auth_xuid,
            "user_type" => self.user_type.as_str(),
            "version_type" => &self.version_type,
            "resolution_width" => &self.resolution_width,
            "resolution_height" => &self.resolution_height,
            "quickPlayPath" => self.quick_play_path.as_deref()?,
            "quickPlaySingleplayer" => self.quick_play_singleplayer.as_deref()?,
            "quickPlayMultiplayer" => self.quick_play_multiplayer.as_deref()?,
            "quickPlayRealms" => self.quick_play_realms.as_deref()?,
            "natives_directory" => &self.natives_directory,
            "launcher_name" => &self.launcher_name,
            "launcher_version" => &self.launcher_version,
            "library_directory" => &self.library_directory,
            "classpath_separator" => &self.classpath_separator,
            "classpath" => &self.classpath,
            _ => return None,
        })
    }
}
