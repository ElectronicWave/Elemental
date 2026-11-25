use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use elemental_core::auth::credential::{Credential, CredentialKind};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::error::{Error, Result};

use super::{
    classpath::{classpath_separator, join_classpath, resolve_classpath},
    layout::InstanceStorage,
    rules::VersionJsonRuleContext,
};

#[derive(Clone, Debug)]
pub struct LauncherVariables {
    auth_player_name: String,
    version_name: String,
    game_directory: String,
    assets_root: String,
    assets_index_name: String,
    auth_uuid: String,
    auth_access_token: String,
    // Legacy 1.7-era launchers still expect this placeholder to be a JSON object string.
    user_properties: String,
    clientid: String,
    auth_xuid: String,
    user_type: UserType,
    version_type: String,
    resolution_width: String,
    resolution_height: String,
    quick_play_path: Option<String>,
    quick_play_singleplayer: Option<String>,
    quick_play_multiplayer: Option<String>,
    quick_play_realms: Option<String>,
    natives_directory: String,
    launcher_name: String,
    launcher_version: String,
    library_directory: String,
    classpath_separator: String,
    classpath: String,
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
    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.clientid = client_id;
        self
    }

    pub fn with_resolution(mut self, width: String, height: String) -> Self {
        self.resolution_width = width;
        self.resolution_height = height;
        self
    }

    pub fn with_launcher(mut self, name: String, version: String) -> Self {
        self.launcher_name = name;
        self.launcher_version = version;
        self
    }

    pub fn with_quick_play(
        mut self,
        path: Option<String>,
        singleplayer: Option<String>,
        multiplayer: Option<String>,
        realms: Option<String>,
    ) -> Self {
        self.quick_play_path = path;
        self.quick_play_singleplayer = singleplayer;
        self.quick_play_multiplayer = multiplayer;
        self.quick_play_realms = realms;
        self
    }

    pub(crate) fn apply_credential(&mut self, credential: &Credential) {
        if self.auth_player_name.is_empty() {
            self.auth_player_name = credential.username.clone();
        }
        self.auth_uuid = credential.uuid.clone();
        self.auth_access_token = credential.access_token.clone();
        self.user_type = match credential.kind {
            CredentialKind::Legacy => UserType::Legacy,
            CredentialKind::Mojang => UserType::Mojang,
            CredentialKind::Msa => UserType::Msa,
        };
        if self.clientid.is_empty() {
            self.clientid = credential.client_id.clone().unwrap_or_default();
        }
        if self.auth_xuid.is_empty() {
            self.auth_xuid = credential.xuid.clone().unwrap_or_default();
        }
    }

    pub(crate) fn apply_metadata(
        &mut self,
        version: &InstanceStorage,
        metadata: &PistonMetaData,
    ) -> Result<()> {
        self.version_name = version.name().ok_or_else(|| Error::MissingVersionName {
            path: version.path.clone(),
        })?;
        self.assets_index_name = metadata.assets.clone();
        self.version_type = metadata.release_type.clone();
        Ok(())
    }

    pub(crate) fn apply_paths(
        &mut self,
        game_directory: &Path,
        assets_root: &Path,
        library_directory: &Path,
        natives_directory: &Path,
    ) {
        self.game_directory = game_directory.to_string_lossy().to_string();
        self.assets_root = assets_root.to_string_lossy().to_string();
        self.library_directory = library_directory.to_string_lossy().to_string();
        self.classpath_separator = classpath_separator().to_owned();
        self.natives_directory = natives_directory.to_string_lossy().to_string();
    }

    pub(crate) fn populate_classpath(
        &mut self,
        version: &InstanceStorage,
        metadata: &PistonMetaData,
        rule_context: &VersionJsonRuleContext,
        module_path_entries: &HashSet<String>,
    ) -> Result<()> {
        let classpath = resolve_classpath(version, metadata, rule_context, module_path_entries)?;
        self.classpath = join_classpath(
            classpath
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
        );
        Ok(())
    }

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

    pub fn apply(&self, args: &[String]) -> Vec<String> {
        args.iter()
            .map(|value| replace_variables(value, self, None))
            .collect()
    }

    pub fn apply_with(
        &self,
        args: &[String],
        extra_variables: &HashMap<String, String>,
    ) -> Vec<String> {
        args.iter()
            .map(|value| replace_variables(value, self, Some(extra_variables)))
            .collect()
    }
}

fn replace_variables(
    value: &str,
    variables: &LauncherVariables,
    extra_variables: Option<&HashMap<String, String>>,
) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find('}') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let key = &after_open[..end];
        let replacement = extra_variables
            .and_then(|extra| extra.get(key).map(String::as_str))
            .or_else(|| variables.get(key));
        match replacement {
            Some(replacement) => output.push_str(replacement),
            None => output.push_str(&rest[start..start + 2 + end + 1]),
        }
        rest = &after_open[end + 1..];
    }

    output.push_str(rest);
    output
}
