use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LaunchEnvs {
    pub auth_player_name: String,
    pub version_name: String,
    pub game_directory: String,
    pub assets_root: String,
    pub assets_index_name: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    pub clientid: String,
    pub auth_xuid: String,
    pub user_type: String,
    pub version_type: String,
    pub resolution_width: String,
    pub resolution_height: String,
    #[serde(rename = "quickPlayPath")]
    pub quick_play_path: String,
    #[serde(rename = "quickPlaySingleplayer")]
    pub quick_play_singleplayer: String,
    #[serde(rename = "quickPlayMultiplayer")]
    pub quick_play_multiplayer: String,
    #[serde(rename = "quickPlayRealms")]
    pub quick_play_realms: String,
    pub natives_directory: String,
    pub launcher_name: String,
    pub launcher_version: String,
    pub classpath: String,
}
