use elemental_schema::maven::MavenMetadataBody;
use quick_xml::de::from_str;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::http::fetch_text;

use super::MavenNotation;
use super::validation::validate_component;

#[derive(Debug, Deserialize)]
pub(crate) struct SnapshotMavenMetadata {
    #[serde(default)]
    version: String,
    versioning: SnapshotVersioning,
}

#[derive(Debug, Deserialize)]
struct SnapshotVersioning {
    snapshot: Option<SnapshotVersion>,
    #[serde(rename = "snapshotVersions", default)]
    snapshot_versions: Vec<SnapshotArtifactVersion>,
}

#[derive(Debug, Deserialize)]
struct SnapshotVersion {
    timestamp: String,
    #[serde(rename = "buildNumber")]
    build_number: usize,
}

#[derive(Debug, Deserialize)]
struct SnapshotArtifactVersion {
    classifier: Option<String>,
    extension: String,
    value: String,
}

impl MavenNotation {
    fn matches_snapshot_artifact(&self, artifact: &SnapshotArtifactVersion) -> bool {
        artifact.extension.as_str() == self.extension.as_str()
            && artifact.classifier.as_deref() == self.classifier.as_deref()
    }
}

impl super::MavenRepository {
    pub(crate) fn resolve_snapshot_version(
        &self,
        notation: &MavenNotation,
        metadata: &SnapshotMavenMetadata,
    ) -> Result<String> {
        if let Some(artifact) = metadata
            .versioning
            .snapshot_versions
            .iter()
            .find(|artifact| notation.matches_snapshot_artifact(artifact))
        {
            validate_component(
                artifact.value.as_str(),
                "maven snapshot version",
                artifact.value.as_str(),
            )?;
            return Ok(artifact.value.clone());
        }

        if let Some(snapshot) = &metadata.versioning.snapshot
            && !metadata.version.is_empty()
        {
            validate_component(
                metadata.version.as_str(),
                "maven snapshot version",
                metadata.version.as_str(),
            )?;
            validate_component(
                snapshot.timestamp.as_str(),
                "maven snapshot timestamp",
                snapshot.timestamp.as_str(),
            )?;
            return Ok(format!(
                "{}-{}-{}",
                metadata.version, snapshot.timestamp, snapshot.build_number
            ));
        }

        Ok(notation.version().to_owned())
    }
}

pub(crate) fn decode_snapshot_metadata(raw: &str, url: String) -> Result<SnapshotMavenMetadata> {
    from_str(raw).map_err(|source| Error::DecodeMavenMetadata {
        source_label: "maven snapshot metadata".to_owned(),
        url,
        source,
    })
}

pub async fn fetch_maven_metadata(
    client: &reqwest::Client,
    url: String,
    source_label: &str,
) -> Result<MavenMetadataBody> {
    let raw = fetch_text(client, url.as_str(), source_label).await?;
    from_str(&raw).map_err(|source| Error::DecodeMavenMetadata {
        source_label: source_label.to_owned(),
        url,
        source,
    })
}
