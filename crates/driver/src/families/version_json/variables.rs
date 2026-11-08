use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};

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

impl LauncherVariables {
    pub fn hashmap(&self) -> Result<HashMap<String, String>> {
        Ok(HashMap::from_iter(self.map()?.into_iter().filter_map(
            |(key, value)| match value {
                Value::String(string) => Some((key, string)),
                _ => None,
            },
        )))
    }

    pub fn map(&self) -> Result<Map<String, Value>> {
        Ok(serde_json::to_value(self)?
            .as_object()
            .ok_or(Error::LauncherVariablesNotObject)?
            .clone())
    }

    pub fn json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn copy_with(&self, key: String, value: String) -> Result<Self> {
        let mut map = self.map()?;
        map.insert(key, Value::String(value));
        Ok(serde_json::from_value(Value::Object(map))?)
    }

    pub fn copy_with_option(&self, key: String, value: Option<String>) -> Result<Self> {
        if let Some(value) = value {
            self.copy_with(key, value)
        } else {
            let mut map = self.map()?;
            map.remove(&key);
            Ok(serde_json::from_value(Value::Object(map))?)
        }
    }
}
