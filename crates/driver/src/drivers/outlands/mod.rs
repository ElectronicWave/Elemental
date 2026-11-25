use crate::{
    descriptors::OUTLANDS_DRIVER,
    driver::InstallerFamilyDriver,
    drivers::retrofutura::{RetroFuturaLine, retrofutura_spec},
    families::installer::{
        InstallerMavenCatalog, InstallerMavenSource, MavenEndpoints, MavenInstallerSpec,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    url::Origin,
};

pub const OUTLANDS_GAME_VERSION: &str = "1.12.2";

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

impl AsRef<str> for OutlandsOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

const OUTLANDS_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["top.outlands:outlands:"]);
const OUTLANDS_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    OUTLANDS_DRIVER,
    OUTLANDS_LIBRARIES,
    OUTLANDS_LIBRARIES,
    ProfileIdPattern::new("-outlands-"),
);

pub const OUTLANDS_SPEC: MavenInstallerSpec = retrofutura_spec(RetroFuturaLine {
    identity: OUTLANDS_IDENTITY,
    family_name: "outlands",
    source_name: "Outlands source",
    loader_version_prefix: "outlands-",
    canonical_origin: "https://maven.outlands.top/releases/",
    group: "top.outlands",
    artifact: "outlands",
    game_version: OUTLANDS_GAME_VERSION,
});

pub fn endpoints() -> MavenEndpoints {
    MavenEndpoints::official(&OUTLANDS_SPEC)
}

pub fn source() -> InstallerMavenSource {
    InstallerMavenSource::official(&OUTLANDS_SPEC)
}

pub fn driver() -> InstallerFamilyDriver {
    InstallerFamilyDriver::for_spec(&OUTLANDS_SPEC)
}

pub fn catalog() -> InstallerMavenCatalog {
    InstallerMavenCatalog::for_spec(&OUTLANDS_SPEC)
}
