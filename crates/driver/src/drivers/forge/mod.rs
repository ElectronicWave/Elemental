use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{LoaderRelease, MavenCatalog},
    descriptors::FORGE_DRIVER,
    error::Result,
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerFamilyDriverSpec, InstallerMavenSource,
        MavenEndpoints, MavenSpec, PreparedInstallerFamilyVersion,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::{Origin, OriginEndpoints},
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
    type Release = LoaderRelease;

    const SOURCE_NAME: &'static str = "forge source";
    const MAVEN_ORIGIN: ForgeOrigin = ForgeOrigin;
    const GROUP_PATH: &'static str = "net/minecraftforge/forge";
    const COORDINATE_GROUP: &'static str = "net.minecraftforge";
    const ARTIFACT: &'static str = "forge";

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        let (game_version, loader_version) = version.split_once('-')?;
        let game_version = MinecraftVersionId::from(game_version);
        Some((
            game_version.clone(),
            LoaderRelease::new(game_version, LoaderVersionId::from(loader_version)),
        ))
    }

    fn rewrite_upstream(endpoints: &OriginEndpoints<ForgeOrigin>, raw_url: &str) -> Result<String> {
        endpoints.rewrite_upstream(raw_url)
    }
}

pub type ForgeEndpoints = MavenEndpoints<ForgeSpec>;
pub type ForgeSource = InstallerMavenSource<ForgeEndpoints>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForgeFamily;

impl InstallerFamily for ForgeFamily {
    type Source = ForgeSource;
    type Endpoints = ForgeEndpoints;

    const FAMILY_NAME: &'static str = "forge";
    const LOADER_VERSION_PREFIX: &'static str = "forge-";
}

pub type ForgeDriver = InstallerFamilyDriver<ForgeFamily>;
pub type PreparedForgeVersion<L, VL> = PreparedInstallerFamilyVersion<ForgeFamily, L, VL>;

const FORGE_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["net.minecraftforge:fmlloader:"]);
const FORGE_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    FORGE_DRIVER,
    FORGE_LIBRARIES,
    FORGE_LIBRARIES,
    ProfileIdPattern::new("-forge-"),
);

impl InstallerFamilyDriverSpec for ForgeFamily {
    const IDENTITY: ProfiledDriverIdentity = FORGE_IDENTITY;
}

pub type ForgeCatalog = MavenCatalog<ForgeEndpoints>;
