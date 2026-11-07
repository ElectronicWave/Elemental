use elemental_schema::maven::MavenMetadataBody;
use quick_xml::de::from_str;
use reqwest::Url;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::http::fetch_text;

/// Maven coordinate in launcher notation:
/// `group:artifact:version[:classifier][@extension]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenNotation {
    group: String,
    artifact: String,
    version: String,
    classifier: Option<String>,
    extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenArtifactDownload {
    pub path: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct MavenRepository {
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct SnapshotMavenMetadata {
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
    pub fn parse(notation: &str, invalid_label: &str) -> Result<Self> {
        let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
        let segments = coordinates.split(':').collect::<Vec<&str>>();

        let (group, artifact, version, classifier) = match segments.as_slice() {
            [group, artifact, version] => (*group, *artifact, *version, None),
            [group, artifact, version, classifier] => {
                (*group, *artifact, *version, Some(*classifier))
            }
            _ => {
                return Err(Error::InvalidNotation {
                    label: invalid_label.to_owned(),
                    notation: notation.to_owned(),
                });
            }
        };

        Ok(Self {
            group: group.to_owned(),
            artifact: artifact.to_owned(),
            version: version.to_owned(),
            classifier: classifier.map(ToOwned::to_owned),
            extension: extension.to_owned(),
        })
    }

    pub fn group(&self) -> &str {
        self.group.as_str()
    }

    fn version(&self) -> &str {
        self.version.as_str()
    }

    pub fn with_classifier(&self, classifier: &str) -> Self {
        Self {
            group: self.group.clone(),
            artifact: self.artifact.clone(),
            version: self.version.clone(),
            classifier: Some(classifier.to_owned()),
            extension: self.extension.clone(),
        }
    }

    pub fn version_directory_path(&self) -> String {
        format!(
            "{}/{}/{}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        )
    }

    fn file_name_for_version(&self, resolved_version: &str) -> String {
        match self.classifier.as_deref() {
            Some(classifier) => format!(
                "{}-{resolved_version}-{classifier}.{}",
                self.artifact, self.extension
            ),
            None => format!("{}-{resolved_version}.{}", self.artifact, self.extension),
        }
    }

    fn matches_snapshot_artifact(&self, artifact: &SnapshotArtifactVersion) -> bool {
        artifact.extension.as_str() == self.extension.as_str()
            && artifact.classifier.as_deref() == self.classifier.as_deref()
    }

    pub fn artifact_path(&self) -> String {
        format!(
            "{}/{}",
            self.version_directory_path(),
            self.file_name_for_version(self.version())
        )
    }
}

impl MavenRepository {
    pub fn parse(base_url: &str, invalid_label: &str) -> Result<Self> {
        let normalized_base_url = ensure_trailing_slash(base_url);
        Url::parse(normalized_base_url.as_str()).map_err(|source| {
            Error::invalid_url(
                format!("parse {invalid_label} url failed"),
                base_url,
                source,
            )
        })?;

        Ok(Self {
            base_url: normalized_base_url,
        })
    }

    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    pub fn artifact_download(&self, artifact_path: String) -> Result<MavenArtifactDownload> {
        let url = artifact_url(self.base_url(), artifact_path.as_str())?;
        Ok(MavenArtifactDownload {
            path: artifact_path,
            url,
        })
    }

    pub async fn resolve_snapshot_artifact(
        &self,
        client: &reqwest::Client,
        notation: &MavenNotation,
    ) -> Result<MavenArtifactDownload> {
        let metadata_url = format!(
            "{}/{}/maven-metadata.xml",
            self.base_url.trim_end_matches('/'),
            notation.version_directory_path()
        );
        let raw_metadata =
            fetch_text(client, metadata_url.as_str(), "maven snapshot metadata").await?;
        let metadata: SnapshotMavenMetadata =
            from_str(raw_metadata.as_str()).map_err(|source| Error::DecodeMavenMetadata {
                source_label: "maven snapshot metadata".to_owned(),
                url: metadata_url,
                source,
            })?;
        let resolved_version = resolve_snapshot_version(notation, &metadata);
        let file_name = notation.file_name_for_version(resolved_version.as_str());
        let path = format!("{}/{}", notation.version_directory_path(), file_name);
        let url = artifact_url(self.base_url(), path.as_str())?;

        Ok(MavenArtifactDownload { path, url })
    }
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

fn artifact_url(base_url: &str, artifact_path: &str) -> Result<String> {
    let normalized_base_url = ensure_trailing_slash(base_url);
    let base = Url::parse(normalized_base_url.as_str()).map_err(|source| {
        Error::invalid_url("parse maven repository url failed", base_url, source)
    })?;

    base.join(artifact_path)
        .map_err(|source| {
            Error::join_url(
                "join maven artifact url failed",
                base_url,
                artifact_path,
                source,
            )
        })
        .map(|url| url.to_string())
}

fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        return url.to_owned();
    }

    format!("{url}/")
}

fn resolve_snapshot_version(notation: &MavenNotation, metadata: &SnapshotMavenMetadata) -> String {
    if let Some(artifact) = metadata
        .versioning
        .snapshot_versions
        .iter()
        .find(|artifact| notation.matches_snapshot_artifact(artifact))
    {
        return artifact.value.clone();
    }

    if let Some(snapshot) = &metadata.versioning.snapshot
        && !metadata.version.is_empty()
    {
        return format!(
            "{}-{}-{}",
            metadata.version, snapshot.timestamp, snapshot.build_number
        );
    }

    notation.version().to_owned()
}
