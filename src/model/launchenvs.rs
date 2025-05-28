use crate::offline;
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
    pub clientid: Option<String>,
    pub auth_xuid: Option<String>,
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

impl LaunchEnvs {
    pub fn offline_player(player_name: String) -> Self {
        Self {
            auth_xuid: None,
            auth_uuid: offline::uuid::player_uuid(&player_name),
            auth_player_name: player_name,
            version_name: todo!(),
            game_directory: todo!(),
            assets_root: todo!(),
            assets_index_name: todo!(),
            auth_access_token: todo!(),
            clientid: None,
            user_type: todo!(),
            version_type: todo!(),
            resolution_width: todo!(),
            resolution_height: todo!(),
            quick_play_path: todo!(),
            quick_play_singleplayer: todo!(),
            quick_play_multiplayer: todo!(),
            quick_play_realms: todo!(),
            natives_directory: todo!(),
            launcher_name: todo!(),
            launcher_version: todo!(),
            classpath: todo!(),
        }
    }
}
