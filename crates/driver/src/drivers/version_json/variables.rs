use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{
    classpath::join_classpath, extensions::PistonMetaLibrariesExt, meta::PistonMetaData,
    rules::VersionJsonRuleContext,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct LauncherVariables {
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
            clientid: Default::default(),
            auth_xuid: Default::default(),
            user_type: UserType::Msa,
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
    Legacy,
    Msa,
    Mojang,
}

impl LauncherVariables {
    pub fn offline_player(
        player_name: String,
        game_directory: String,
        version_root: String,
        natives_directory: String,
        pistonmeta: &PistonMetaData,
    ) -> Result<Self> {
        let rule_context = VersionJsonRuleContext::current();
        let version_name = Path::new(&version_root)
            .file_name()
            .context("version root has no file name")?
            .to_string_lossy()
            .to_string();
        let version_jar = Path::new(&version_root).join(format!("{version_name}.jar"));
        let classpath = pistonmeta
            .libraries
            .iter()
            .filter(|library| library.is_allowed(&rule_context))
            .filter_map(|library| {
                let artifact = library.downloads.artifact.as_ref()?;
                if artifact.path.contains("natives") {
                    return None;
                }

                Some(
                    Path::new(&game_directory)
                        .join("libraries")
                        .join(&artifact.path)
                        .to_string_lossy()
                        .to_string(),
                )
            })
            .chain(std::iter::once(version_jar.to_string_lossy().to_string()))
            .collect::<Vec<String>>();
        let classpath = join_classpath(classpath);

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
            user_type: UserType::Legacy,
            version_type: pistonmeta.release_type.clone(),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: None,
            quick_play_singleplayer: None,
            quick_play_multiplayer: None,
            quick_play_realms: None,
            natives_directory,
            launcher_name: "Elemental".to_owned(),
            launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
            classpath,
        })
    }

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
            .context("struct is not an object")?
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

    pub fn apply_mut(&self, args: &mut [String]) -> Result<()> {
        let data = self.map()?;
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;

        for value in args.iter_mut() {
            let original = value.clone();
            let mut copied = original.clone();
            for variable in regex.captures_iter(&original) {
                if let Some(key) = variable.get(1).map(|item| item.as_str()) {
                    if let Some(Value::String(replacement)) = data.get(key) {
                        copied = copied.replace(&format!("${{{key}}}"), replacement);
                    }
                }
            }
            *value = copied;
        }

        Ok(())
    }

    pub fn apply(&self, args: Vec<String>) -> Result<Vec<String>> {
        self.apply_with(args, &HashMap::new())
    }

    pub fn apply_with(
        &self,
        args: Vec<String>,
        extra_variables: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let mut result = Vec::with_capacity(args.len());
        let mut data = self.hashmap()?;
        data.extend(extra_variables.clone());
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;

        for value in &args {
            let mut copied = value.clone();
            for variable in regex.captures_iter(value) {
                if let Some(key) = variable.get(1).map(|item| item.as_str()) {
                    if let Some(replacement) = data.get(key) {
                        copied = copied.replace(&format!("${{{key}}}"), replacement);
                    }
                }
            }
            result.push(copied);
        }

        Ok(result)
    }
}
