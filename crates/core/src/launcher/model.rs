use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{consts::PLATFORM_NATIVES_DIR_NAME, models::mojang::PistonMetaData};

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
    pub classpath: String,
}

impl Default for LaunchEnvs {
    fn default() -> Self {
        Self {
            auth_player_name: Default::default(),
            version_name: Default::default(),
            game_directory: Default::default(),
            assets_root: Default::default(),
            assets_index_name: Default::default(),
            auth_uuid: Default::default(),
            auth_access_token: Default::default(),
            clientid: Default::default(),
            auth_xuid: Default::default(),
            user_type: UserType::MSA,
            version_type: Default::default(),
            resolution_width: Default::default(),
            resolution_height: Default::default(),
            quick_play_path: Default::default(),
            quick_play_singleplayer: Default::default(),
            quick_play_multiplayer: Default::default(),
            quick_play_realms: Default::default(),
            natives_directory: Default::default(),
            launcher_name: Default::default(),
            launcher_version: Default::default(),
            classpath: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserType {
    LEGACY,
    MSA,
    MOJANG,
}

impl LaunchEnvs {
    pub fn offline_player(
        player_name: String,
        game_directory: String,
        version_root: String,
        pistonmeta: &PistonMetaData,
    ) -> Result<Self> {
        let version_name = Path::new(&version_root)
            .file_name()
            .context("version root has no file name")?
            .to_string_lossy()
            .to_string();
        let version_jar = Path::new(&version_root).join(format!("{version_name}.jar"));
        let classpath = pistonmeta
            .libraries
            .iter()
            .filter_map(|library| {
                if library.downloads.artifact.path.contains("natives") {
                    None
                } else {
                    Some(
                        Path::new(&game_directory)
                            .join("libraries")
                            .join(&library.downloads.artifact.path)
                            .to_string_lossy()
                            .to_string(),
                    )
                }
            })
            .chain(std::iter::once(version_jar.to_string_lossy().to_string()))
            .collect::<Vec<String>>()
            .join(";");

        Ok(Self {
            auth_player_name: player_name,
            version_name,
            game_directory: game_directory.clone(),
            assets_root: Path::new(&game_directory)
                .join("assets")
                .to_string_lossy()
                .to_string(),
            assets_index_name: pistonmeta.assets.clone(),
            auth_uuid: String::new(),
            auth_access_token: String::new(),
            clientid: String::new(),
            auth_xuid: String::new(),
            user_type: UserType::LEGACY,
            version_type: pistonmeta.release_type.clone(),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: None,
            quick_play_singleplayer: None,
            quick_play_multiplayer: None,
            quick_play_realms: None,
            natives_directory: Path::new(&version_root)
                .join(PLATFORM_NATIVES_DIR_NAME)
                .to_string_lossy()
                .to_string(),
            launcher_name: "Elemental".to_owned(),
            launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
            classpath,
        })
    }

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
            .context("struct is not a object.")?
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

    pub fn apply_launchenvs_mut(&self, args: &mut Vec<String>) -> Result<()> {
        let data = self.map()?;
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
        self.apply_launchenvs_with(args, &HashMap::new())
    }

    pub fn apply_launchenvs_with(
        &self,
        args: Vec<String>,
        extra_variables: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let mut result = vec![];
        let mut data = self.hashmap()?;
        data.extend(extra_variables.clone());
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;
        for value in args.iter() {
            let mut copied = value.clone();
            for var in regex.captures_iter(value) {
                if let Some(key) = var.get(1).map(|e| e.as_str()) {
                    if let Some(val) = data.get(key) {
                        copied = copied.replace(&format!("${{{key}}}"), val)
                    }
                }
            }
            result.push(copied);
        }
        Ok(result)
    }
}
