use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::asset_index::PistonMetaAssetIndex;

/// https://piston-meta.mojang.com/v1/packages/<sha1>/<id>.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<PistonMetaArguments>,
    #[serde(
        rename = "minecraftArguments",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub minecraft_arguments: Option<String>,
    #[serde(
        rename = "inheritsFrom",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub inherits_from: Option<String>,
    #[serde(
        rename = "clientVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub client_version: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default)]
    pub game: Vec<PistonMetaGenericArgument>,
    #[serde(default)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ContinuousArgument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaRuleArgumentRules {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<HashMap<String, bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<OperatingSystem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OperatingSystem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    pub name: String,
    #[serde(flatten)]
    pub source: PistonMetaLibrariesSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PistonMetaRuleArgumentRules>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<PistonMetaLibrariesExtract>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PistonMetaLibrariesSource {
    Downloads {
        downloads: PistonMetaLibrariesDownloads,
        #[serde(default, flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    Maven {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha1: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<usize>,
        #[serde(default, flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibrariesExtract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibrariesDownloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<PistonMetaLibrariesDownloadsArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifiers: Option<HashMap<String, PistonMetaLibrariesDownloadsArtifact>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLibrariesDownloadsArtifact {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
    pub url: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaLogging {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<PistonMetaLoggingSide>,
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
