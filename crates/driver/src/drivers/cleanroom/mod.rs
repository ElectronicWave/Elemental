use crate::{
    catalog::InstallerMavenCatalog,
    descriptors::CLEANROOM_DRIVER,
    drivers::retrofutura::{RetroFuturaLine, retrofutura_spec},
    families::installer::{
        InstallerFamilyDriver, InstallerMavenSource, MavenEndpoints, MavenInstallerSpec,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    url::Origin,
};

pub const CLEANROOM_GAME_VERSION: &str = "1.12.2";

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

impl AsRef<str> for CleanroomOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

const CLEANROOM_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["com.cleanroommc:cleanroom:"]);
const CLEANROOM_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    CLEANROOM_DRIVER,
    CLEANROOM_LIBRARIES,
    CLEANROOM_LIBRARIES,
    ProfileIdPattern::new("-cleanroom-"),
);

pub const CLEANROOM_SPEC: MavenInstallerSpec = retrofutura_spec(RetroFuturaLine {
    identity: CLEANROOM_IDENTITY,
    family_name: "cleanroom",
    source_name: "cleanroom source",
    loader_version_prefix: "cleanroom-",
    canonical_origin: "https://repo.cleanroommc.com/releases",
    group: "com.cleanroommc",
    artifact: "cleanroom",
    game_version: CLEANROOM_GAME_VERSION,
});

pub fn endpoints() -> MavenEndpoints {
    MavenEndpoints::official(&CLEANROOM_SPEC)
}

pub fn source() -> InstallerMavenSource {
    InstallerMavenSource::official(&CLEANROOM_SPEC)
}

pub fn driver() -> InstallerFamilyDriver {
    InstallerFamilyDriver::for_spec(&CLEANROOM_SPEC)
}

pub fn catalog() -> InstallerMavenCatalog {
    InstallerMavenCatalog::for_spec(&CLEANROOM_SPEC)
}
