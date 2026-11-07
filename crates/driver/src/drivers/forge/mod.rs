use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{InstallerMavenCatalog, LoaderRelease},
    descriptors::FORGE_DRIVER,
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerMavenSource, MavenEndpoints, MavenSpec,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::Origin,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForgeOrigin;

impl Origin for ForgeOrigin {
    fn canonical(self) -> &'static str {
        "https://maven.minecraftforge.net"
    }

    fn all() -> &'static [Self] {
        &[Self]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ForgeSpec;

impl MavenSpec for ForgeSpec {
    type Origin = ForgeOrigin;

    const SOURCE_NAME: &'static str = "forge source";
    const MAVEN_ORIGIN: ForgeOrigin = ForgeOrigin;
    const GROUP_PATH: &'static str = "net/minecraftforge";
    const COORDINATE_GROUP: &'static str = "net.minecraftforge";
    const ARTIFACT: &'static str = "forge";

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        let mut parts = version.split('-');
        let game_version = parts.next()?;
        let loader_version = parts.next()?;
        let game_version = MinecraftVersionId::from(game_version);
        Some((
            game_version.clone(),
            LoaderRelease::new(game_version, LoaderVersionId::from(loader_version)),
        ))
    }
}

pub type ForgeEndpoints = MavenEndpoints<ForgeSpec>;
pub type ForgeSource = InstallerMavenSource<ForgeEndpoints>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForgeFamily;

impl InstallerFamily for ForgeFamily {
    type Source = ForgeSource;
    type Endpoints = ForgeEndpoints;

    const IDENTITY: ProfiledDriverIdentity = FORGE_IDENTITY;
    const FAMILY_NAME: &'static str = "forge";
    const LOADER_VERSION_PREFIX: &'static str = "forge-";
}

pub type ForgeDriver = InstallerFamilyDriver<ForgeFamily>;

const FORGE_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["net.minecraftforge:fmlloader:"]);
const FORGE_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    FORGE_DRIVER,
    FORGE_LIBRARIES,
    FORGE_LIBRARIES,
    ProfileIdPattern::new("-forge-"),
);

pub type ForgeCatalog = InstallerMavenCatalog<ForgeEndpoints>;
