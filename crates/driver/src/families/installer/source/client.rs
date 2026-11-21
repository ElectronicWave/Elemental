use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::maven::MavenMetadataBody;

use crate::{
    error::Result,
    families::{
        installer::{
            InstallerArtifact, MavenEndpoints, MavenInstallerSpec, build_installer_artifact,
        },
        version_json::VersionJsonRootLayout,
    },
    http::HttpSource,
    loader_version::LoaderVersionId,
    maven::fetch_maven_metadata,
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

    pub fn spec(&self) -> &'static MavenInstallerSpec {
        self.inner.endpoints().spec()
    }

    pub fn endpoints(&self) -> &MavenEndpoints {
        self.inner.endpoints()
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints().maven_metadata_url()?;
        fetch_maven_metadata(self.inner.client()?, url, self.spec().source_name).await
    }

    pub fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
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
