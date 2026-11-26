#[cfg(feature = "maven-xml")]
use std::collections::HashSet;

#[cfg(feature = "maven-xml")]
use async_trait::async_trait;
use elemental_core::{http::SwappableClient, minecraft::MinecraftVersionId};
#[cfg(feature = "maven-xml")]
use elemental_schema::maven::MavenMetadataBody;
use elemental_schema::mojang::piston::PistonMetaLibraries;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

#[cfg(feature = "maven-xml")]
use crate::catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog};
#[cfg(feature = "maven-xml")]
use crate::maven::fetch_maven_metadata;
use crate::{
    error::Result,
    families::{
        installer::{
            InstallerArtifact, MavenEndpoints, MavenInstallerSpec, build_installer_artifact,
        },
        version_json::GameStorage,
    },
    http::HttpClient,
    loader_version::LoaderVersionId,
    url::OriginPolicy,
};

#[derive(Debug, Clone)]
pub struct InstallerMavenSource {
    #[cfg_attr(not(feature = "maven-xml"), allow(dead_code))]
    client: HttpClient,
    endpoints: MavenEndpoints,
}

impl InstallerMavenSource {
    pub fn new(endpoints: MavenEndpoints) -> Self {
        let source_name = endpoints.spec().source_name;
        Self {
            client: HttpClient::new(source_name),
            endpoints,
        }
    }

    pub fn official(spec: &'static MavenInstallerSpec) -> Self {
        Self::new(MavenEndpoints::official(spec))
    }

    pub fn with_policy(spec: &'static MavenInstallerSpec, origin_policy: OriginPolicy) -> Self {
        Self::new(MavenEndpoints::new(spec, origin_policy))
    }

    pub fn with_client(endpoints: MavenEndpoints, client: reqwest::Client) -> Self {
        Self {
            client: HttpClient::with_client(client),
            endpoints,
        }
    }

    pub fn with_http_client(endpoints: MavenEndpoints, client: SwappableClient) -> Self {
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

    pub fn spec(&self) -> &'static MavenInstallerSpec {
        self.endpoints.spec()
    }

    pub fn endpoints(&self) -> &MavenEndpoints {
        &self.endpoints
    }

    pub fn normalize_libraries(
        &self,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>> {
        self.endpoints.normalize_libraries(libraries)
    }

    #[cfg(feature = "maven-xml")]
    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints.maven_metadata_url()?;
        fetch_maven_metadata(&self.client, url, self.spec().source_name).await
    }

    pub fn installer_artifact(
        &self,
        game_storage: &GameStorage,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact> {
        let spec = self
            .endpoints
            .installer_artifact_spec(game_version, loader_version)?;

        build_installer_artifact(
            game_storage,
            spec.coordinate,
            spec.download_url,
            spec.relative_path,
        )
    }
}

#[cfg(feature = "maven-xml")]
pub struct InstallerMavenCatalog {
    source: InstallerMavenSource,
}

#[cfg(feature = "maven-xml")]
impl InstallerMavenCatalog {
    pub fn new(source: InstallerMavenSource) -> Self {
        Self { source }
    }

    pub fn for_spec(spec: &'static MavenInstallerSpec) -> Self {
        Self::new(InstallerMavenSource::official(spec))
    }

    pub fn source(&self) -> &InstallerMavenSource {
        &self.source
    }
}

#[cfg(feature = "maven-xml")]
#[async_trait]
impl GameVersionCatalog for InstallerMavenCatalog {
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        let spec = self.source.spec();
        if let Some(fixed_game_version) = spec.fixed_game_version {
            return Ok(vec![GameVersion {
                id: MinecraftVersionId::from(fixed_game_version),
                channel: None,
            }]);
        }

        let metadata = self.source.maven_metadata().await?;
        let mut seen = HashSet::new();
        let mut game_versions = Vec::new();
        for version in metadata.versioning.versions.version {
            let Some((game_version, release)) = spec.parse_release(version) else {
                continue;
            };
            if seen.insert(game_version.clone()) {
                game_versions.push(GameVersion {
                    id: game_version,
                    channel: release.channel,
                });
            }
        }
        Ok(game_versions)
    }
}

#[cfg(feature = "maven-xml")]
#[async_trait]
impl LoaderVersionCatalog for InstallerMavenCatalog {
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        let spec = self.source.spec();
        let metadata = self.source.maven_metadata().await?;
        Ok(metadata
            .versioning
            .versions
            .version
            .into_iter()
            .filter_map(|version| spec.parse_release(version))
            .filter(|(_, release)| &release.game_version == game_version)
            .map(|(_, release)| release)
            .collect())
    }
}
