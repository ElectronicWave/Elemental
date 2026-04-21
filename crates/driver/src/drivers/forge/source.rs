use std::path::PathBuf;

use crate::{
    families::{
        installer::InstallerArtifact,
        version_json::{VersionJsonRootLayout, VersionJsonRootResource},
    },
    http::{build_default_client, fetch_text},
    url::{Origin, OriginPolicy},
};
use anyhow::{Context, Result};
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_schema::forge::MavenMetadataBody;
use quick_xml::de::from_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForgeOrigin {
    Maven,
}

#[derive(Debug, Clone)]
pub struct ForgeEndpoints {
    origin_policy: OriginPolicy<ForgeOrigin>,
}

#[derive(Debug, Clone)]
pub struct ForgeSource {
    client: reqwest::Client,
    endpoints: ForgeEndpoints,
}

impl Origin for ForgeOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Maven => "https://maven.minecraftforge.net",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[ForgeOrigin] = &[ForgeOrigin::Maven];
        ALL
    }
}

impl Default for ForgeEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl ForgeEndpoints {
    pub fn new(origin_policy: OriginPolicy<ForgeOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<ForgeOrigin> {
        &self.origin_policy
    }

    pub fn maven_metadata_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            ForgeOrigin::Maven,
            "/net/minecraftforge/forge/maven-metadata.xml",
        )
    }

    pub fn installer_url(&self, game_version: &str, loader_version: &str) -> Result<String> {
        let version = release_version(game_version, loader_version);
        self.origin_policy.resolve(
            ForgeOrigin::Maven,
            &format!("/net/minecraftforge/forge/{version}/forge-{version}-installer.jar"),
        )
    }

    pub fn maven_artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.origin_policy
            .resolve(ForgeOrigin::Maven, artifact_path)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_origin_url(raw_url)
    }
}

impl Default for ForgeSource {
    fn default() -> Self {
        Self {
            client: build_default_client("forge source"),
            endpoints: ForgeEndpoints::default(),
        }
    }
}

impl ForgeSource {
    pub fn new(endpoints: ForgeEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn endpoints(&self) -> &ForgeEndpoints {
        &self.endpoints
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints.maven_metadata_url()?;
        let raw = fetch_text(&self.client, url.as_str(), "forge source").await?;

        from_str(&raw).with_context(|| format!("decode forge maven metadata failed: {url}"))
    }

    pub fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        game_version: &str,
        loader_version: &str,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        let version = release_version(game_version, loader_version);
        let library_relative_path = forge_installer_relative_path(&version);

        Ok(InstallerArtifact {
            coordinate: format!("net.minecraftforge:forge:{version}:installer"),
            url: self.endpoints.installer_url(game_version, loader_version)?,
            path: game_storage.try_get_resource(VersionJsonRootResource::Libraries(Some(
                library_relative_path,
            )))?,
            expected_size: None,
            sha1: None,
        })
    }
}

pub fn release_version(game_version: &str, loader_version: &str) -> String {
    format!("{game_version}-{loader_version}")
}

pub fn forge_installer_relative_path(version: &str) -> PathBuf {
    PathBuf::from("net")
        .join("minecraftforge")
        .join("forge")
        .join(version)
        .join(format!("forge-{version}-installer.jar"))
}

pub fn parse_installer_version(version: &str) -> Result<(String, String)> {
    version
        .split_once('-')
        .map(|(game_version, loader_version)| (game_version.to_owned(), loader_version.to_owned()))
        .context(format!("forge installer version is invalid: {version}"))
}
