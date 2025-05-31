use crate::{error::unification::UnifiedResult, offline};
use serde::{Deserialize, Serialize};
use std::io::Result;

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
    pub classpath: String,
}

impl LaunchEnvs {
    pub fn json(&self) -> Result<String> {
        serde_json::to_string(self).to_stdio()
    }

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
            user_type: "msa".to_owned(),
            version_type: "Elemental ...".to_owned(),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: None,
            quick_play_singleplayer: None,
            quick_play_multiplayer: None,
            quick_play_realms: None,
            natives_directory: todo!(),
            launcher_name: "Elemental".to_owned(),
            launcher_version: "todo!()".to_owned(),
            classpath: todo!(),
        }
    }
}
