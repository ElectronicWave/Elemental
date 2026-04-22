use anyhow::{Context, Result, bail};
use quick_xml::de::from_str;
use reqwest::Url;
use serde::Deserialize;

use crate::http::fetch_text;

#[derive(Debug, Deserialize)]
struct SnapshotMavenMetadata {
    versioning: SnapshotVersioning,
}

#[derive(Debug, Deserialize)]
struct SnapshotVersioning {
    snapshot: SnapshotVersion,
}

#[derive(Debug, Deserialize)]
struct SnapshotVersion {
    timestamp: String,
    #[serde(rename = "buildNumber")]
    build_number: usize,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedMavenNotation {
    group: String,
    artifact: String,
    version: String,
    classifier: Option<String>,
    extension: String,
}

#[derive(Debug, Clone)]
pub(super) struct ResolvedArtifactDownload {
    pub(super) path: String,
    pub(super) url: String,
}

impl ParsedMavenNotation {
    pub(super) fn version(&self) -> &str {
        self.version.as_str()
    }

    pub(super) fn version_directory_path(&self) -> String {
        format!(
            "{}/{}/{}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        )
    }

    fn file_name_for_version(&self, resolved_version: &str) -> String {
        match self.classifier.as_deref() {
            Some(classifier) => {
                format!(
                    "{}-{resolved_version}-{classifier}.{}",
                    self.artifact, self.extension
                )
            }
            None => format!("{}-{resolved_version}.{}", self.artifact, self.extension),
        }
    }
}

pub(super) async fn resolve_snapshot_artifact(
    client: &reqwest::Client,
    repository_base_url: &str,
    notation: &ParsedMavenNotation,
) -> Result<ResolvedArtifactDownload> {
    let metadata_url = format!(
        "{}/{}/maven-metadata.xml",
        repository_base_url.trim_end_matches('/'),
        notation.version_directory_path()
    );
    let raw_metadata = fetch_text(
        client,
        metadata_url.as_str(),
        "liteloader snapshot metadata",
    )
    .await?;
    let metadata: SnapshotMavenMetadata = from_str(raw_metadata.as_str())
        .with_context(|| format!("decode liteloader snapshot metadata failed: {metadata_url}"))?;
    // Snapshot repositories publish timestamped files instead of the plain -SNAPSHOT artifact name.
    let resolved_version = format!(
        "{}-{}-{}",
        notation.version().trim_end_matches("-SNAPSHOT"),
        metadata.versioning.snapshot.timestamp,
        metadata.versioning.snapshot.build_number
    );
    let file_name = notation.file_name_for_version(resolved_version.as_str());
    let path = format!("{}/{}", notation.version_directory_path(), file_name);
    let url = join_artifact_url(repository_base_url, path.as_str())?;

    Ok(ResolvedArtifactDownload { path, url })
}

pub(super) fn parse_maven_notation(
    notation: &str,
    invalid_label: &str,
) -> Result<ParsedMavenNotation> {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    let segments = coordinates.split(':').collect::<Vec<&str>>();

    let (group, artifact, version, classifier) = match segments.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] => (*group, *artifact, *version, Some(*classifier)),
        _ => bail!("invalid {invalid_label} notation: {notation}"),
    };

    Ok(ParsedMavenNotation {
        group: group.to_owned(),
        artifact: artifact.to_owned(),
        version: version.to_owned(),
        classifier: classifier.map(ToOwned::to_owned),
        extension: extension.to_owned(),
    })
}

pub(super) fn join_artifact_url(repository_base_url: &str, artifact_path: &str) -> Result<String> {
    let base =
        Url::parse(ensure_trailing_slash(repository_base_url).as_str()).with_context(|| {
            format!("parse LiteLoader repository url failed: {repository_base_url}")
        })?;

    base.join(artifact_path)
        .with_context(|| {
            format!(
                "join LiteLoader repository artifact url failed: {repository_base_url} + {artifact_path}"
            )
        })
        .map(|url| url.to_string())
}

pub(super) fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        return url.to_owned();
    }

    format!("{url}/")
}

pub(super) fn is_snapshot_version(version: &str) -> bool {
    version.ends_with("-SNAPSHOT")
}

pub(super) fn is_snapshot_version_from_notation(notation: &str) -> bool {
    notation.split(':').nth(2).is_some_and(is_snapshot_version)
}
