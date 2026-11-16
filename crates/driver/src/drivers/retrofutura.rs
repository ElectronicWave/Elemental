use std::marker::PhantomData;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::{
    catalog::{
        GameVersion, GameVersionCatalog, InstallerMavenCatalog, LoaderRelease, LoaderVersionCatalog,
    },
    error::Result,
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerMavenSource, MavenEndpoints, MavenSpec,
        merge_retro_futura_libraries,
    },
    inspect::ProfiledDriverIdentity,
    loader_version::LoaderVersionId,
    url::Origin,
};

/// Shared declaration for loaders that replace the runtime of an old Minecraft
/// version while keeping that version's game metadata as the base.
pub trait RetroFuturaCatalog: Copy + std::fmt::Debug + Send + Sync + 'static {
    type Origin: Origin;

    const ORIGIN: Self::Origin;
    const SOURCE_NAME: &'static str;
    const GROUP: &'static str;
    const ARTIFACT: &'static str;
    const GAME_VERSION: &'static str;
    const IDENTITY: ProfiledDriverIdentity;
    const FAMILY_NAME: &'static str;
    const LOADER_VERSION_PREFIX: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct RetroFuturaMaven<C>(PhantomData<C>);

impl<C> MavenSpec for RetroFuturaMaven<C>
where
    C: RetroFuturaCatalog,
{
    type Origin = C::Origin;

    const SOURCE_NAME: &'static str = C::SOURCE_NAME;
    const MAVEN_ORIGIN: C::Origin = C::ORIGIN;
    const GROUP: &'static str = C::GROUP;
    const ARTIFACT: &'static str = C::ARTIFACT;

    fn version(_game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        loader_version.as_str().to_owned()
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        let game_version = MinecraftVersionId::from(C::GAME_VERSION);
        Some((
            game_version.clone(),
            LoaderRelease::new(game_version, LoaderVersionId::from(version)),
        ))
    }
}

pub type RetroFuturaEndpoints<C> = MavenEndpoints<RetroFuturaMaven<C>>;
pub type RetroFuturaSource<C> = InstallerMavenSource<RetroFuturaEndpoints<C>>;

pub struct RetroFuturaReleaseCatalog<C>
where
    C: RetroFuturaCatalog,
{
    inner: InstallerMavenCatalog<RetroFuturaEndpoints<C>>,
}

impl<C> RetroFuturaReleaseCatalog<C>
where
    C: RetroFuturaCatalog,
{
    pub fn new(source: RetroFuturaSource<C>) -> Self {
        Self {
            inner: InstallerMavenCatalog::new(source),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RetroFuturaSource::<C>::default())
    }

    pub fn source(&self) -> &RetroFuturaSource<C> {
        self.inner.source()
    }
}

#[async_trait]
impl<C> GameVersionCatalog for RetroFuturaReleaseCatalog<C>
where
    C: RetroFuturaCatalog,
{
    async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        Ok(vec![GameVersion {
            id: MinecraftVersionId::from(C::GAME_VERSION),
            channel: None,
        }])
    }
}

#[async_trait]
impl<C> LoaderVersionCatalog for RetroFuturaReleaseCatalog<C>
where
    C: RetroFuturaCatalog,
{
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        self.inner.loader_versions(game_version).await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RetroFuturaFamily<C>(PhantomData<C>);

impl<C> InstallerFamily for RetroFuturaFamily<C>
where
    C: RetroFuturaCatalog,
{
    type Source = RetroFuturaSource<C>;
    type Endpoints = RetroFuturaEndpoints<C>;

    const IDENTITY: ProfiledDriverIdentity = C::IDENTITY;
    const FAMILY_NAME: &'static str = C::FAMILY_NAME;
    const LOADER_VERSION_PREFIX: &'static str = C::LOADER_VERSION_PREFIX;

    fn merge_libraries(
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        merge_retro_futura_libraries(base_libraries, embedded_libraries)
    }
}

pub type RetroFuturaDriver<C> = InstallerFamilyDriver<RetroFuturaFamily<C>>;
