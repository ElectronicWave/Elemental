use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "metadata")]
pub struct MavenMetadataBody {
    pub versioning: MavenMetadataVersioning,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MavenMetadataVersioning {
    pub versions: MavenMetadataVersion,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MavenMetadataVersion {
    pub version: Vec<String>,
}
