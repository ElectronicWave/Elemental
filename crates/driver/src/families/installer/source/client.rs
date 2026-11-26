use elemental_core::{http::SwappableClient, minecraft::MinecraftVersionId};
#[cfg(feature = "maven-xml")]
use elemental_schema::maven::MavenMetadataBody;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

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
    http::HttpSource,
    loader_version::LoaderVersionId,
    url::OriginPolicy,
};

#[derive(Debug, Clone)]
pub struct InstallerMavenSource {
    inner: HttpSource<MavenEndpoints>,
}

impl InstallerMavenSource {
    pub fn new(endpoints: MavenEndpoints) -> Self {
        let source_name = endpoints.spec().source_name;
        Self {
            inner: HttpSource::new(endpoints, source_name),
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
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn with_http_client(endpoints: MavenEndpoints, client: SwappableClient) -> Self {
        Self {
            inner: HttpSource::with_http_client(endpoints, client),
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.inner = self.inner.with_headers(headers);
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.inner = self.inner.with_header(name, value);
        self
    }

    pub fn spec(&self) -> &'static MavenInstallerSpec {
        self.inner.endpoints().spec()
    }

    pub fn endpoints(&self) -> &MavenEndpoints {
        self.inner.endpoints()
    }

    #[cfg(feature = "maven-xml")]
    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints().maven_metadata_url()?;
        let client = self.inner.client()?;
        fetch_maven_metadata(&client, url, self.spec().source_name, self.inner.headers()).await
    }

    pub fn installer_artifact(
        &self,
        game_storage: &GameStorage,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact> {
        let spec = self
            .endpoints()
            .installer_artifact_spec(game_version, loader_version)?;

        build_installer_artifact(
            game_storage,
            spec.coordinate,
            spec.download_url,
            spec.relative_path,
        )
    }
}
