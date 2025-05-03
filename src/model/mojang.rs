use std::collections::HashMap;

use serde::Deserialize;

pub struct MojangBaseUrl {
    pub launchermeta: String,
    pub launchermeta_https: bool,
    pub pistonmeta: String,
}

impl Default for MojangBaseUrl {
    fn default() -> Self {
        Self {
            launchermeta: "launchermeta.mojang.com".to_owned(),
            launchermeta_https: false,
            pistonmeta: "piston-meta.mojang.com".to_owned(),
        }
    }
}

/// http://launchermeta.mojang.com/mc/game/version_manifest.json
#[derive(Debug, Deserialize)]
pub struct LaunchMetaData {
    pub latest: LaunchMetaLatestData,
    pub versions: Vec<LaunchMetaVersionData>,
}

#[derive(Debug, Deserialize)]
pub struct LaunchMetaLatestData {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Deserialize)]
pub struct LaunchMetaVersionData {
    pub id: String,
    #[serde(rename = "type")]
    pub typo: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// https://piston-meta.mojang.com/v1/packages/<->/<->.json
#[derive(Debug, Deserialize)]
pub struct PistonMetaData {
    pub arguments: PistonMetaArguments,
    #[serde(rename = "assetIndex")]
    pub asset_index: PistonMetaAssetIndex,
    pub assets: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: usize,
    pub downloads: PistonMetaDownloads,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: PistonMetaJavaVersion,
    //TODO pub libraries: ...
}

#[derive(Debug, Deserialize)]
pub struct PistonMetaArguments {
    pub game: Vec<PistonMetaGenericArgument>,
    pub jvm: Vec<PistonMetaGenericArgument>,
}
#[derive(Debug, Deserialize)]
pub enum PistonMetaGenericArgument {
    Plain(String),
    Rule(PistonMetaRuleArgument),
}

#[derive(Debug, Deserialize)]
pub struct PistonMetaRuleArgument {
    pub rules: Vec<PistonMetaRuleArgumentRules>,
    pub value: ContinuousArgument,
}

#[derive(Debug, Deserialize)]
pub struct PistonMetaRuleArgumentRules {
    pub action: String,
    pub features: Option<HashMap<String, String>>,
    pub os: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub enum ContinuousArgument {
    Single(String),
    Multi(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct PistonMetaAssetIndex {
    pub id: String,
    pub sha1: String,
    // size may be too small
    pub size: usize,
    #[serde(rename = "totalSize")]
    pub total_size: usize,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PistonMetaDownloads {
    pub client: PistonMetaDownload,
    pub server: PistonMetaDownload,
}

#[derive(Debug, Deserialize)]
pub struct PistonMetaDownload {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
#[derive(Debug, Deserialize)]
pub struct PistonMetaJavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: usize,
}
