use serde::{Deserialize, Serialize};

pub use crate::fabric::{
    ProfileJson, ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads,
    ProfileLibraryExtract, ProfileLogging, ProfileLoggingClient, ProfileLoggingFile,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderVersionHashes {
    pub sha1: String,
    pub sha256: String,
    pub sha512: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderVersion {
    pub maven: String,
    pub version: String,
    pub build: i32,
    pub separator: String,
    #[serde(rename = "file_size")]
    pub file_size: usize,
    pub hashes: LoaderVersionHashes,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HashedVersion {
    pub maven: String,
    pub version: String,
    #[serde(rename = "file_size")]
    pub file_size: usize,
    pub hashes: LoaderVersionHashes,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntermediaryVersion {
    pub maven: String,
    pub version: String,
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
    #[serde(default)]
    pub development: Vec<LauncherLibrary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherMainClass {
    pub client: String,
    pub server: String,
    #[serde(rename = "serverLauncher")]
    pub server_launcher: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LauncherMeta {
    pub version: usize,
    pub libraries: LauncherLibraries,
    #[serde(rename = "mainClass")]
    pub main_class: LauncherMainClass,
    #[serde(rename = "min_java_version")]
    pub min_java_version: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderGameVersion {
    pub loader: LoaderVersion,
    pub hashed: HashedVersion,
    pub intermediary: IntermediaryVersion,
    #[serde(rename = "launcherMeta")]
    pub launcher_meta: LauncherMeta,
}
