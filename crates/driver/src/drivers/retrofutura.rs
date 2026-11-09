use std::marker::PhantomData;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::{
    catalog::{InstallerMavenCatalog, LoaderRelease},
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
    const RELEASE_DESCRIPTION: &'static str;
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
            LoaderRelease::new(game_version, LoaderVersionId::from(version))
                .with_description(Some(C::RELEASE_DESCRIPTION.to_owned())),
        ))
    }
}

pub type RetroFuturaEndpoints<C> = MavenEndpoints<RetroFuturaMaven<C>>;
pub type RetroFuturaSource<C> = InstallerMavenSource<RetroFuturaEndpoints<C>>;
pub type RetroFuturaReleaseCatalog<C> = InstallerMavenCatalog<RetroFuturaEndpoints<C>>;

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
