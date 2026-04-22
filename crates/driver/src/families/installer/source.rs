use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::forge::MavenMetadataBody;

use crate::{
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
