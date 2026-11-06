use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{LoaderRelease, MavenCatalog},
    descriptors::OUTLANDS_DRIVER,
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerFamilyDriverSpec, InstallerMavenSource,
        MavenEndpoints, MavenSpec, PreparedInstallerFamilyVersion,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::Origin,
};
use elemental_schema::mojang::piston::PistonMetaLibraries;

pub const OUTLANDS_GAME_VERSION: &str = "1.12.2";
const OUTLANDS_RELEASE_DESCRIPTION: &str = "Outlands installer releases target Minecraft 1.12.2.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutlandsOrigin;

impl Origin for OutlandsOrigin {
    fn canonical(self) -> &'static str {
        "https://maven.outlands.top/releases/"
    }

    fn all() -> &'static [Self] {
        &[Self]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OutlandsSpec;

impl MavenSpec for OutlandsSpec {
    type Origin = OutlandsOrigin;

    const SOURCE_NAME: &'static str = "Outlands source";
    const MAVEN_ORIGIN: OutlandsOrigin = OutlandsOrigin;
    const GROUP_PATH: &'static str = "top/outlands";
    const COORDINATE_GROUP: &'static str = "top.outlands";
    const ARTIFACT: &'static str = "outlands";

    fn version(_game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        loader_version.as_str().to_owned()
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        let game_version = MinecraftVersionId::from(OUTLANDS_GAME_VERSION);
        Some((
            game_version.clone(),
            LoaderRelease::new(game_version, LoaderVersionId::from(version))
                .with_description(Some(OUTLANDS_RELEASE_DESCRIPTION.to_owned())),
        ))
    }
}

pub type OutlandsEndpoints = MavenEndpoints<OutlandsSpec>;
pub type OutlandsSource = InstallerMavenSource<OutlandsEndpoints>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutlandsDriverSpec;

impl InstallerFamily for OutlandsDriverSpec {
    type Source = OutlandsSource;
    type Endpoints = OutlandsEndpoints;

    const FAMILY_NAME: &'static str = "outlands";
    const LOADER_VERSION_PREFIX: &'static str = "outlands-";

    fn merge_libraries(
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        crate::families::installer::merge_retro_futura_libraries(base_libraries, embedded_libraries)
    }
}

pub type OutlandsDriver = InstallerFamilyDriver<OutlandsDriverSpec>;
pub type PreparedOutlandsVersion<L, VL> = PreparedInstallerFamilyVersion<OutlandsDriverSpec, L, VL>;

const OUTLANDS_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["top.outlands:outlands:"]);
const OUTLANDS_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    OUTLANDS_DRIVER,
    OUTLANDS_LIBRARIES,
    OUTLANDS_LIBRARIES,
    ProfileIdPattern::new("-outlands-"),
);

impl InstallerFamilyDriverSpec for OutlandsDriverSpec {
    const IDENTITY: ProfiledDriverIdentity = OUTLANDS_IDENTITY;
}

pub type OutlandsCatalog = MavenCatalog<OutlandsEndpoints>;
