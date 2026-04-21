use std::{path::PathBuf, time::Duration};

use crate::{
    families::{installer::InstallerArtifact, version_json::VersionJsonGameStorageExt},
    url::{Origin, OriginPolicy},
};
use anyhow::{Context, Result};
use elemental_core::storage::Storage;
use elemental_schema::forge::MavenMetadataBody;
use quick_xml::de::from_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeoForgeOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct NeoForgeEndpoints {
    origin_policy: OriginPolicy<NeoForgeOrigin>,
}

#[derive(Debug, Clone)]
pub struct NeoForgeSource {
    client: reqwest::Client,
    endpoints: NeoForgeEndpoints,
}

impl Origin for NeoForgeOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Maven => "https://maven.neoforged.net/releases",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[NeoForgeOrigin] = &[NeoForgeOrigin::Maven];
        ALL
    }
}

impl Default for NeoForgeEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl NeoForgeEndpoints {
    pub fn new(origin_policy: OriginPolicy<NeoForgeOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<NeoForgeOrigin> {
        &self.origin_policy
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            NeoForgeOrigin::Maven,
            "/net/neoforged/neoforge/maven-metadata.xml",
        )
    }

    pub fn installer_url(&self, loader_version: &str) -> Result<String> {
        let version = release_version(loader_version);
        self.origin_policy.resolve(
            NeoForgeOrigin::Maven,
            &format!("/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"),
        )
    }

    pub fn maven_artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.origin_policy
            .resolve(NeoForgeOrigin::Maven, artifact_path)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.origin_policy.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}

impl Default for NeoForgeSource {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("build neoforge source client failed"),
            endpoints: NeoForgeEndpoints::default(),
        }
    }
}

impl NeoForgeSource {
    pub fn new(endpoints: NeoForgeEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn endpoints(&self) -> &NeoForgeEndpoints {
        &self.endpoints
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints.maven_metadata_url()?;
        let raw = self
            .client
            .get(url.as_str())
            .send()
            .await
            .with_context(|| format!("request neoforge source resource failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("neoforge source returned error status: {url}"))?
            .text()
            .await
            .with_context(|| format!("decode neoforge source resource failed: {url}"))?;

        from_str(&raw).with_context(|| format!("decode neoforge maven metadata failed: {url}"))
    }

    pub fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        _game_version: &str,
        loader_version: &str,
    ) -> Result<InstallerArtifact>
    where
        L: crate::families::version_json::VersionJsonRootLayout,
    {
        let version = release_version(loader_version);
        let library_relative_path = neoforge_installer_relative_path(&version);

        Ok(InstallerArtifact {
            coordinate: format!("net.neoforged:neoforge:{version}:installer"),
            url: self.endpoints.installer_url(loader_version)?,
            path: game_storage.library_path(&library_relative_path)?,
            expected_size: None,
            sha1: None,
        })
    }
}

pub fn release_version(loader_version: &str) -> String {
    loader_version.to_owned()
}

pub fn neoforge_installer_relative_path(version: &str) -> PathBuf {
    PathBuf::from("net")
        .join("neoforged")
        .join("neoforge")
        .join(version)
        .join(format!("neoforge-{version}-installer.jar"))
}
