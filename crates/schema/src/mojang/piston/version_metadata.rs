use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::asset_index::PistonMetaAssetIndex;

/// https://piston-meta.mojang.com/v1/packages/<sha1>/<id>.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaData {
    pub arguments: Option<PistonMetaArguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "inheritsFrom", default)]
    pub inherits_from: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: PistonMetaAssetIndex,
    pub assets: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: usize,
    pub downloads: PistonMetaDownloads,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: PistonMetaJavaVersion,
    pub libraries: Vec<PistonMetaLibraries>,
    pub logging: Option<PistonMetaLogging>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaArguments {
    pub game: Vec<PistonMetaGenericArgument>,
    pub jvm: Vec<PistonMetaGenericArgument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PistonMetaGenericArgument {
    Plain(String),
    Rule(PistonMetaRuleArgument),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaRuleArgument {
    pub rules: Vec<PistonMetaRuleArgumentRules>,
    pub value: Option<ContinuousArgument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaRuleArgumentRules {
    pub action: String,
    pub features: Option<HashMap<String, bool>>,
    pub os: Option<OperatingSystem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OperatingSystem {
    pub arch: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContinuousArgument {
    Single(String),
    Multi(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaDownloads {
    pub client: PistonMetaDownload,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaDownload {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaJavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibraries {
    pub downloads: PistonMetaLibrariesDownloads,
    pub name: String,
    pub natives: Option<HashMap<String, String>>,
    pub rules: Option<Vec<PistonMetaRuleArgumentRules>>,
    pub extract: Option<PistonMetaLibrariesExtract>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibrariesExtract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibrariesDownloads {
    pub artifact: PistonMetaLibrariesDownloadsArtifact,
    pub classifiers: Option<HashMap<String, PistonMetaLibrariesDownloadsArtifact>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibrariesDownloadsArtifact {
    pub sha1: Option<String>,
    pub size: Option<usize>,
    pub url: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLogging {
    pub client: PistonMetaLoggingSide,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLoggingSide {
    pub argument: String,
    pub file: PistonMetaLoggingSideFile,
    #[serde(rename = "type")]
    pub logging_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLoggingSideFile {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
