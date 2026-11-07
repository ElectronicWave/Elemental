use elemental_infra::SafeRelativePath;
use reqwest::Url;

use crate::error::{Error, Result};
use crate::http::fetch_text;
use crate::url::ensure_directory_url;

use super::snapshot::decode_snapshot_metadata;
use super::url::{artifact_url, ensure_http_scheme};
use super::{MavenArtifactDownload, MavenNotation};

#[derive(Debug, Clone)]
pub struct MavenRepository {
    base_url: Url,
}

impl MavenRepository {
    pub fn parse(base_url: &str, invalid_label: &str) -> Result<Self> {
        let parsed = Url::parse(base_url).map_err(|source| {
            Error::invalid_url(
                format!("parse {invalid_label} url failed"),
                base_url,
                source,
            )
        })?;
        ensure_http_scheme(&parsed, format!("{invalid_label} url"))?;
        let base_url = ensure_directory_url(parsed)?;

        Ok(Self { base_url })
    }

    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    pub fn artifact_download(
        &self,
        artifact_path: SafeRelativePath,
    ) -> Result<MavenArtifactDownload> {
        let url = artifact_url(&self.base_url, &artifact_path)?;
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
        let version_directory = notation.version_directory_path()?;
        let metadata_path =
            SafeRelativePath::parse(format!("{}/maven-metadata.xml", version_directory.as_str()))?;
        let metadata_url = artifact_url(&self.base_url, &metadata_path)?;
        let raw_metadata =
            fetch_text(client, metadata_url.as_str(), "maven snapshot metadata").await?;
        let metadata = decode_snapshot_metadata(raw_metadata.as_str(), metadata_url)?;
        let resolved_version = self.resolve_snapshot_version(notation, &metadata)?;
        let file_name = notation.file_name_for_version(resolved_version.as_str())?;
        let path =
            SafeRelativePath::parse(format!("{}/{}", version_directory.as_str(), file_name))?;
        let url = artifact_url(&self.base_url, &path)?;

        Ok(MavenArtifactDownload { path, url })
    }
}
