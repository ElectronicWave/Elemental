use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::mojang::piston::PistonMetaLibraries;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeInstallerProfile {
    #[serde(default)]
    pub spec: Option<usize>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub json: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub minecraft: Option<String>,
    #[serde(default)]
    pub data: HashMap<String, ForgeInstallerDataEntry>,
    #[serde(default)]
    pub processors: Vec<ForgeInstallerProcessor>,
    #[serde(default)]
    pub libraries: Vec<PistonMetaLibraries>,
    #[serde(rename = "mirrorList", default)]
    pub mirror_list: Option<String>,
    #[serde(default)]
    pub install: Option<ForgeInstallerLegacyInstall>,
    #[serde(rename = "versionInfo", default)]
    pub version_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeInstallerDataEntry {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeInstallerProcessor {
    pub jar: String,
    #[serde(default)]
    pub classpath: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    #[serde(default)]
    pub sides: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForgeInstallerLegacyInstall {
    #[serde(rename = "profileName", default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "filePath", default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub welcome: Option<String>,
    #[serde(default)]
    pub minecraft: Option<String>,
    #[serde(rename = "mirrorList", default)]
    pub mirror_list: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
}
