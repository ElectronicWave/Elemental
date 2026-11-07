use std::{fmt::Debug, marker::PhantomData, path::PathBuf};

use crate::error::Result;
use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::maven::MavenMetadataBody;

use crate::{
    catalog::LoaderRelease,
    families::{
        installer::{
            InstallerArtifact, InstallerArtifactEndpoints, InstallerArtifactSource,
            build_installer_artifact,
        },
        version_json::VersionJsonRootLayout,
    },
    http::HttpSource,
    loader_version::LoaderVersionId,
    maven::fetch_maven_metadata,
    url::{Origin, OriginEndpoints},
};

#[derive(Debug, Clone)]
pub struct InstallerMavenArtifactSpec {
    pub coordinate: String,
    pub download_url: String,
    pub relative_path: PathBuf,
}

pub trait InstallerMavenEndpoints:
    InstallerArtifactEndpoints + Clone + Debug + Send + Sync + 'static
{
    const SOURCE_NAME: &'static str;

    fn maven_metadata_url(&self) -> Result<String>;

    fn installer_artifact_spec(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec>;

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)>;
}

pub trait MavenSpec: Clone + Debug + Send + Sync + 'static {
    type Origin: Origin;

    const SOURCE_NAME: &'static str;
    const MAVEN_ORIGIN: Self::Origin;
    const GROUP_PATH: &'static str;
    const COORDINATE_GROUP: &'static str;
    const ARTIFACT: &'static str;

    fn version(game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        format!("{}-{}", game_version.as_str(), loader_version.as_str())
    }

    fn metadata_path() -> String {
        format!(
            "/{}/{}/maven-metadata.xml",
            Self::GROUP_PATH,
            Self::ARTIFACT
        )
    }

    fn artifact_path(version: &str) -> String {
        format!(
            "/{}/{}/{version}/{}-{version}-installer.jar",
            Self::GROUP_PATH,
            Self::ARTIFACT,
            Self::ARTIFACT,
        )
    }

    fn installer_artifact(
        endpoints: &OriginEndpoints<Self::Origin>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        let version = Self::version(game_version, loader_version);
        let artifact_path = Self::artifact_path(&version);
        Ok(InstallerMavenArtifactSpec {
            coordinate: format!(
                "{}:{}:{version}:installer",
                Self::COORDINATE_GROUP,
                Self::ARTIFACT
            ),
            download_url: endpoints.resolve(Self::MAVEN_ORIGIN, &artifact_path)?,
            relative_path: PathBuf::from(artifact_path.trim_start_matches('/')),
        })
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)>;

    fn rewrite_upstream(
        endpoints: &OriginEndpoints<Self::Origin>,
        raw_url: &str,
    ) -> Result<String> {
        endpoints.rewrite_known_or_keep(raw_url)
    }
}

#[derive(Debug, Clone)]
pub struct MavenEndpoints<S>
where
    S: MavenSpec,
{
    endpoints: OriginEndpoints<S::Origin>,
    marker: PhantomData<S>,
}

impl<S> Default for MavenEndpoints<S>
where
    S: MavenSpec,
{
    fn default() -> Self {
        Self::official()
    }
}

impl<S> MavenEndpoints<S>
where
    S: MavenSpec,
{
    pub fn new(endpoints: OriginEndpoints<S::Origin>) -> Self {
        Self {
            endpoints,
            marker: PhantomData,
        }
    }

    pub fn official() -> Self {
        Self::new(OriginEndpoints::official())
    }

    pub fn resolve(&self, path: &str) -> Result<String> {
        self.endpoints.resolve(S::MAVEN_ORIGIN, path)
    }

    pub fn origin_policy(&self) -> &crate::url::OriginPolicy<S::Origin> {
        self.endpoints.origin_policy()
    }
}

impl<S> InstallerArtifactEndpoints for MavenEndpoints<S>
where
    S: MavenSpec,
{
    fn artifact_url(&self, artifact_path: &str) -> Result<String> {
        self.resolve(artifact_path)
    }

    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        S::rewrite_upstream(&self.endpoints, raw_url)
    }
}

impl<S> InstallerMavenEndpoints for MavenEndpoints<S>
where
    S: MavenSpec,
{
    const SOURCE_NAME: &'static str = S::SOURCE_NAME;

    fn maven_metadata_url(&self) -> Result<String> {
        self.resolve(&S::metadata_path())
    }

    fn installer_artifact_spec(
        &self,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerMavenArtifactSpec> {
        S::installer_artifact(&self.endpoints, game_version, loader_version)
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        S::release(version)
    }
}

#[derive(Debug, Clone)]
pub struct InstallerMavenSource<E>
where
    E: InstallerMavenEndpoints,
{
    inner: HttpSource<E>,
}

impl<E> Default for InstallerMavenSource<E>
where
    E: InstallerMavenEndpoints + Default,
{
    fn default() -> Self {
        Self::new(E::default())
    }
}

impl<E> InstallerMavenSource<E>
where
    E: InstallerMavenEndpoints,
{
    pub fn new(endpoints: E) -> Self {
        Self {
            inner: HttpSource::new(endpoints, E::SOURCE_NAME),
        }
    }

    pub fn with_client(endpoints: E, client: reqwest::Client) -> Self {
        Self {
            inner: HttpSource::with_client(endpoints, client),
        }
    }

    pub fn endpoints(&self) -> &E {
        self.inner.endpoints()
    }

    pub async fn maven_metadata(&self) -> Result<MavenMetadataBody> {
        let url = self.endpoints().maven_metadata_url()?;
        fetch_maven_metadata(self.inner.client(), url, E::SOURCE_NAME).await
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

impl<E> InstallerArtifactSource for InstallerMavenSource<E>
where
    E: InstallerMavenEndpoints,
{
    type Endpoints = E;

    fn endpoints(&self) -> &Self::Endpoints {
        InstallerMavenSource::endpoints(self)
    }

    fn installer_artifact<L>(
        &self,
        game_storage: &Storage<L>,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        InstallerMavenSource::installer_artifact(self, game_storage, game_version, loader_version)
    }
}
