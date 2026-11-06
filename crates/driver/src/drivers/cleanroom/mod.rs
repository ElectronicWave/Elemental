use elemental_core::minecraft::MinecraftVersionId;

use crate::{
    catalog::{LoaderRelease, MavenCatalog},
    descriptors::CLEANROOM_DRIVER,
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerFamilyDriverSpec, InstallerMavenSource,
        MavenEndpoints, MavenSpec, PreparedInstallerFamilyVersion,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    loader_version::LoaderVersionId,
    url::Origin,
};
use elemental_schema::mojang::piston::PistonMetaLibraries;

pub const CLEANROOM_GAME_VERSION: &str = "1.12.2";
const CLEANROOM_RELEASE_DESCRIPTION: &str = "Cleanroom installer releases target Minecraft 1.12.2.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CleanroomOrigin;

impl Origin for CleanroomOrigin {
    fn canonical(self) -> &'static str {
        "https://repo.cleanroommc.com/releases"
    }

    fn all() -> &'static [Self] {
        &[Self]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CleanroomSpec;

impl MavenSpec for CleanroomSpec {
    type Origin = CleanroomOrigin;

    const SOURCE_NAME: &'static str = "cleanroom source";
    const MAVEN_ORIGIN: CleanroomOrigin = CleanroomOrigin;
    const GROUP_PATH: &'static str = "com/cleanroommc";
    const COORDINATE_GROUP: &'static str = "com.cleanroommc";
    const ARTIFACT: &'static str = "cleanroom";

    fn version(_game_version: &MinecraftVersionId, loader_version: &LoaderVersionId) -> String {
        loader_version.as_str().to_owned()
    }

    fn release(version: String) -> Option<(MinecraftVersionId, LoaderRelease)> {
        let game_version = MinecraftVersionId::from(CLEANROOM_GAME_VERSION);
        Some((
            game_version.clone(),
            LoaderRelease::new(game_version, LoaderVersionId::from(version))
                .with_description(Some(CLEANROOM_RELEASE_DESCRIPTION.to_owned())),
        ))
    }
}

pub type CleanroomEndpoints = MavenEndpoints<CleanroomSpec>;
pub type CleanroomSource = InstallerMavenSource<CleanroomEndpoints>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CleanroomDriverSpec;

impl InstallerFamily for CleanroomDriverSpec {
    type Source = CleanroomSource;
    type Endpoints = CleanroomEndpoints;

    const FAMILY_NAME: &'static str = "cleanroom";
    const LOADER_VERSION_PREFIX: &'static str = "cleanroom-";

    fn merge_libraries(
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        crate::families::installer::merge_retro_futura_libraries(base_libraries, embedded_libraries)
    }
}

pub type CleanroomDriver = InstallerFamilyDriver<CleanroomDriverSpec>;
pub type PreparedCleanroomVersion<L, VL> =
    PreparedInstallerFamilyVersion<CleanroomDriverSpec, L, VL>;

const CLEANROOM_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["com.cleanroommc:cleanroom:"]);
const CLEANROOM_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    CLEANROOM_DRIVER,
    CLEANROOM_LIBRARIES,
    CLEANROOM_LIBRARIES,
    ProfileIdPattern::new("-cleanroom-"),
);

impl InstallerFamilyDriverSpec for CleanroomDriverSpec {
    const IDENTITY: ProfiledDriverIdentity = CLEANROOM_IDENTITY;
}

pub type CleanroomCatalog = MavenCatalog<CleanroomEndpoints>;
