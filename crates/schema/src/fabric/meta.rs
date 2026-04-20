use serde::{Deserialize, Serialize};

use crate::mojang::piston::PistonMetaArguments;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderVersion {
    pub separator: String,
    pub build: i32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntermediaryVersion {
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderGameVersion {
    pub loader: LoaderVersion,
    pub intermediary: IntermediaryVersion,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherLibraries {
    pub client: Vec<LauncherLibrary>,
    pub common: Vec<LauncherLibrary>,
    pub server: Vec<LauncherLibrary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherMainClass {
    pub client: String,
    pub server: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherMeta {
    pub version: usize,
    pub libraries: LauncherLibraries,
    #[serde(rename = "mainClass")]
    pub main_class: LauncherMainClass,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderProfile {
    pub loader: LoaderVersion,
    pub intermediary: IntermediaryVersion,
    #[serde(rename = "launcherMeta")]
    pub launcher_meta: LauncherMeta,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileLoggingFile {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileLoggingClient {
    pub argument: String,
    pub file: ProfileLoggingFile,
    #[serde(rename = "type")]
    pub logging_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileLogging {
    pub client: ProfileLoggingClient,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    pub arguments: Option<PistonMetaArguments>,
    pub assets: Option<String>,
    pub libraries: Vec<ProfileLibrary>,
    pub logging: Option<ProfileLogging>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: usize,
    #[serde(rename = "type")]
    pub release_type: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}
