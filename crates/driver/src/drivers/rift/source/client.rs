use elemental_infra::jar::JarBytes;
use elemental_schema::fabric::ProfileJson;

use crate::{
    error::{Error, Result},
    families::version_json::UpstreamUrlRewriter,
    http::{HttpSource, fetch_bytes, fetch_json},
};

use super::endpoints::RiftEndpoints;
use super::profile_normalize::normalize_profile_json;
use super::profile_parse::parse_profile_json;
use super::release::{GitHubRelease, RiftRelease, build_rift_release, release_for_loader_version};

#[derive(Debug, Clone)]
pub struct RiftSource {
    inner: HttpSource<RiftEndpoints>,
}

impl Default for RiftSource {
    fn default() -> Self {
        Self::new(RiftEndpoints::default())
    }
}

impl RiftSource {
    pub fn new(endpoints: RiftEndpoints) -> Self {
        Self {
            inner: HttpSource::new(endpoints, "rift source"),
        }
    }

    pub fn with_client(endpoints: RiftEndpoints, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn endpoints(&self) -> &RiftEndpoints {
        self.inner.endpoints()
    }

    pub async fn releases(&self) -> Result<Vec<RiftRelease>> {
        let url = self.endpoints().releases_url()?;
        let releases: Vec<GitHubRelease> =
            fetch_json(self.inner.client()?, url.as_str(), "rift source").await?;

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
            .endpoints()
            .release_asset_url(release.tag_name.as_str(), release.asset_name.as_str())?;
        let jar_bytes = fetch_bytes(self.inner.client()?, url.as_str(), "rift source").await?;
        let raw_profile = read_release_profile_json(release, jar_bytes)?;
        let profile = parse_profile_json(raw_profile.as_str())?;

        normalize_profile_json(profile, release, self.endpoints())
    }
}

impl UpstreamUrlRewriter for RiftEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        RiftEndpoints::rewrite_upstream(self, raw_url)
    }
}

fn read_release_profile_json(release: &RiftRelease, jar_bytes: Vec<u8>) -> Result<String> {
    JarBytes::new(jar_bytes.as_slice())
        .by_name_string("profile.json")
        .map_err(|source| Error::ReadRiftProfile {
            asset_name: release.asset_name.clone(),
            source,
        })
}
