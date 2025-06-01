use crate::{error::unification::UnifiedResult, offline};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

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

pub enum UserType {
    LEGACY,
    MSA,
    MOJANG,
}

impl ToString for UserType {
    fn to_string(&self) -> String {
        match self {
            UserType::LEGACY => "legacy".to_owned(),
            UserType::MSA => "msa".to_owned(),
            UserType::MOJANG => "mojang".to_owned(),
        }
    }
}

impl LaunchEnvs {
    pub fn hashmap(&self) -> Result<HashMap<String, String>> {
        Ok(HashMap::from_iter(self.map()?.into_iter().filter_map(
            |(k, v)| {
                if let Value::String(val) = v {
                    Some((k, val))
                } else {
                    None
                }
            },
        )))
    }

    pub fn map(&self) -> Result<Map<String, Value>> {
        Ok(serde_json::to_value(self)
            .to_stdio()?
            .as_object()
            .ok_or(Error::new(ErrorKind::Other, "struct is not a object."))?
            .clone())
    }

    pub fn json(&self) -> Result<String> {
        serde_json::to_string(self).to_stdio()
    }

    pub fn copy_with(&self, key: String, value: String) -> Result<Self> {
        let mut map = self.map()?;
        map.insert(key, Value::String(value));
        serde_json::from_value(Value::Object(map)).to_stdio()
    }

    pub fn copy_with_option(&self, key: String, value: Option<String>) -> Result<Self> {
        if let Some(val) = value {
            self.copy_with(key, val)
        } else {
            let mut map = self.map()?;
            map.remove(&key);
            serde_json::from_value(Value::Object(map)).to_stdio()
        }
    }

    pub fn offline_player(player_name: String) -> Self {
        Self {
            auth_xuid: "${auth_xuid}".to_owned(),
            auth_uuid: offline::uuid::player_uuid(&player_name),
            auth_player_name: player_name,
            version_name: "todo!()".to_owned(),
            game_directory: "todo!()".to_owned(),
            assets_root: "todo!()".to_owned(),
            assets_index_name: "todo!()".to_owned(),
            auth_access_token: "todo!()".to_owned(),
            clientid: "${clientid}".to_owned(),
            user_type: "msa".to_owned(),
            version_type: "Elemental".to_owned(),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: None,
            quick_play_singleplayer: None,
            quick_play_multiplayer: None,
            quick_play_realms: None,
            natives_directory: "msa".to_owned(),
            launcher_name: "todo!()".to_owned(),
            launcher_version: "todo!()".to_owned(),
            classpath: "todo!()".to_owned(),
        }
    }
}

#[test]
fn test_useability() {
    let _ = LaunchEnvs::offline_player("Elemental".to_owned())
        .hashmap()
        .unwrap();
}
