use super::mojang::PistonMetaData;
use crate::{consts::PLATFORM_NATIVES_DIR_NAME, offline};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::{Path, absolute},
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
        Ok(serde_json::to_value(self)?
            .as_object()
            .ok_or(Error::new(ErrorKind::Other, "struct is not a object."))?
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
        if let Some(val) = value {
            self.copy_with(key, val)
        } else {
            let mut map = self.map()?;
            map.remove(&key);
            Ok(serde_json::from_value(Value::Object(map))?)
        }
    }

    pub fn offline_player(
        player_name: String,
        storage_root_dir: String,
        version_dir: String,
        version_data: &PistonMetaData,
    ) -> Result<Self> {
        let version_path = absolute(Path::new(&version_dir))?;
        let storage_root = absolute(Path::new(&storage_root_dir))?;
        let version_name = version_path
            .file_name()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or("unknown".to_owned());
        let classpath = version_data
            .libraries
            .iter()
            .filter_map(|e| {
                if e.downloads.artifact.path.contains("natives") {
                    return None;
                }

                Some(
                    storage_root
                        .join("libraries")
                        .join(&e.downloads.artifact.path)
                        .to_string_lossy()
                        .to_string(),
                )
            })
            .collect::<Vec<String>>()
            .join(";")
            + ";"
            + &version_path
                .join(format!("{}.jar", version_name))
                .to_string_lossy()
                .to_string();

        Ok(Self {
            auth_xuid: "${auth_xuid}".to_owned(),
            auth_uuid: offline::uuid::player_uuid(&player_name),
            auth_player_name: player_name,
            version_name: version_name,
            game_directory: version_path.to_string_lossy().to_string(),
            assets_root: storage_root.join("assets").to_string_lossy().to_string(),
            assets_index_name: version_data.assets.clone(),
            auth_access_token: "${auth_access_token}".to_owned(),
            clientid: "${clientid}".to_owned(),
            user_type: "msa".to_owned(),
            version_type: format!("Elemental (Core {})", env!("CARGO_PKG_VERSION")),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: None,
            quick_play_singleplayer: None,
            quick_play_multiplayer: None,
            quick_play_realms: None,
            natives_directory: version_path
                .join(PLATFORM_NATIVES_DIR_NAME)
                .to_string_lossy()
                .to_string(),
            launcher_name: "Elemental".to_owned(), // Let it can be customized
            launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
            classpath: classpath,
        })
    }

    pub fn apply_launchenvs_mut(&self, args: &mut Vec<String>) -> Result<()> {
        let data = self.map()?;
        //TODO Build a algorithm instead of using regex
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;
        for (index, mut copied) in args.clone().into_iter().enumerate() {
            let value = copied.clone();
            for var in regex.captures_iter(&value) {
                if let Some(key) = var.get(1).map(|e| e.as_str()) {
                    if let Some(Value::String(val)) = data.get(key) {
                        copied = copied.replace(&format!("${{{}}}", key), val)
                    }
                }
            }

            args[index] = copied;
        }

        Ok(())
    }

    pub fn apply_launchenvs(&self, args: Vec<String>) -> Result<Vec<String>> {
        let mut result = vec![];
        let data = self.map()?;
        //TODO Build a algorithm instead of using regex
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;
        for value in args.iter() {
            let mut copied = value.clone();
            for var in regex.captures_iter(value) {
                if let Some(key) = var.get(1).map(|e| e.as_str()) {
                    if let Some(Value::String(val)) = data.get(key) {
                        copied = copied.replace(&format!("${{{key}}}"), val)
                    }
                }
            }
            result.push(copied);
        }
        Ok(result)
    }
}
