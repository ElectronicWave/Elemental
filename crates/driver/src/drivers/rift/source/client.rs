use elemental_core::http::SwappableClient;
use elemental_infra::jar::JarBytes;
use elemental_schema::fabric::ProfileJson;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::{
    error::{Error, Result},
    families::version_json::UpstreamUrlRewriter,
    http::HttpClient,
};

use super::endpoints::RiftEndpoints;
use super::profile_normalize::normalize_profile_json;
use super::release::{GitHubRelease, RiftRelease, build_rift_release, release_for_loader_version};

#[derive(Debug, Clone)]
pub struct RiftSource {
    client: HttpClient,
    endpoints: RiftEndpoints,
}

impl Default for RiftSource {
    fn default() -> Self {
        Self::new(RiftEndpoints::default())
    }
}

impl RiftSource {
    pub fn new(endpoints: RiftEndpoints) -> Self {
        Self {
            client: HttpClient::new("rift source"),
            endpoints,
        }
    }

    pub fn with_client(endpoints: RiftEndpoints, client: reqwest::Client) -> Self {
        Self {
            client: HttpClient::with_client(client),
            endpoints,
        }
    }

    pub fn with_http_client(endpoints: RiftEndpoints, client: SwappableClient) -> Self {
        Self {
            client: HttpClient::with_http_client(client),
            endpoints,
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.client = self.client.with_headers(headers);
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.client = self.client.with_header(name, value);
        self
    }

    pub fn endpoints(&self) -> &RiftEndpoints {
        &self.endpoints
    }

    pub async fn releases(&self) -> Result<Vec<RiftRelease>> {
        let url = self.endpoints.releases_url()?;
        let releases: Vec<GitHubRelease> =
            self.client.fetch_json(url.as_str(), "rift source").await?;

        releases
            .into_iter()
            .filter(|release| !release.draft)
            .map(build_rift_release)
            .collect()
    }

    pub async fn profile_json(&self, loader_version: &str) -> Result<ProfileJson> {
        let release = release_for_loader_version(loader_version);
        self.profile_json_for_release(&release).await
    }

    pub async fn profile_json_for_release(&self, release: &RiftRelease) -> Result<ProfileJson> {
        let url = self
            .endpoints
            .release_asset_url(release.tag_name.as_str(), release.asset_name.as_str())?;
        let jar_bytes = self.client.fetch_bytes(url.as_str(), "rift source").await?;
        let raw_profile = read_release_profile_json(release, jar_bytes)?;
        let profile = serde_json::from_slice(raw_profile.as_slice())
            .map_err(|source| Error::DecodeRiftProfile { source })?;

        normalize_profile_json(profile, release, &self.endpoints)
    }
}

impl UpstreamUrlRewriter for RiftEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        RiftEndpoints::rewrite_upstream(self, raw_url)
    }
}

fn read_release_profile_json(release: &RiftRelease, jar_bytes: Vec<u8>) -> Result<Vec<u8>> {
    JarBytes::new(jar_bytes.as_slice())
        .by_name_bytes("profile.json")
        .map_err(|source| Error::ReadRiftProfile {
            asset_name: release.asset_name.clone(),
            source,
        })
}
