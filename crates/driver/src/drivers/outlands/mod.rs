use crate::{
    descriptors::OUTLANDS_DRIVER,
    drivers::retrofutura::{
        RetroFuturaCatalog, RetroFuturaDriver, RetroFuturaEndpoints, RetroFuturaFamily,
        RetroFuturaMaven, RetroFuturaReleaseCatalog, RetroFuturaSource,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    url::Origin,
};

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

const OUTLANDS_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&["top.outlands:outlands:"]);
const OUTLANDS_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    OUTLANDS_DRIVER,
    OUTLANDS_LIBRARIES,
    OUTLANDS_LIBRARIES,
    ProfileIdPattern::new("-outlands-"),
);

#[derive(Debug, Clone, Copy)]
pub struct OutlandsRetroFutura;

impl RetroFuturaCatalog for OutlandsRetroFutura {
    type Origin = OutlandsOrigin;

    const ORIGIN: OutlandsOrigin = OutlandsOrigin;
    const SOURCE_NAME: &'static str = "Outlands source";
    const GROUP_PATH: &'static str = "top/outlands";
    const COORDINATE_GROUP: &'static str = "top.outlands";
    const ARTIFACT: &'static str = "outlands";
    const GAME_VERSION: &'static str = OUTLANDS_GAME_VERSION;
    const RELEASE_DESCRIPTION: &'static str = OUTLANDS_RELEASE_DESCRIPTION;
    const IDENTITY: ProfiledDriverIdentity = OUTLANDS_IDENTITY;
    const FAMILY_NAME: &'static str = "outlands";
    const LOADER_VERSION_PREFIX: &'static str = "outlands-";
}

pub type OutlandsSpec = RetroFuturaMaven<OutlandsRetroFutura>;
pub type OutlandsEndpoints = RetroFuturaEndpoints<OutlandsRetroFutura>;
pub type OutlandsSource = RetroFuturaSource<OutlandsRetroFutura>;
pub type OutlandsDriverSpec = RetroFuturaFamily<OutlandsRetroFutura>;
pub type OutlandsDriver = RetroFuturaDriver<OutlandsRetroFutura>;
pub type OutlandsCatalog = RetroFuturaReleaseCatalog<OutlandsRetroFutura>;
