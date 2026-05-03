use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchMetaData {
    pub latest: LaunchMetaLatestData,
    pub versions: Vec<LaunchMetaVersionData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchMetaLatestData {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchMetaVersionData {
    pub id: String,
    #[serde(rename = "type")]
    pub release_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: DateTime<Utc>,
    pub sha1: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: usize,
}
