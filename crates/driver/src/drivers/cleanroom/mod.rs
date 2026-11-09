use crate::{
    descriptors::CLEANROOM_DRIVER,
    drivers::retrofutura::{
        RetroFuturaCatalog, RetroFuturaDriver, RetroFuturaEndpoints, RetroFuturaFamily,
        RetroFuturaMaven, RetroFuturaReleaseCatalog, RetroFuturaSource,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
    url::Origin,
};

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

const CLEANROOM_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["com.cleanroommc:cleanroom:"]);
const CLEANROOM_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    CLEANROOM_DRIVER,
    CLEANROOM_LIBRARIES,
    CLEANROOM_LIBRARIES,
    ProfileIdPattern::new("-cleanroom-"),
);

#[derive(Debug, Clone, Copy)]
pub struct CleanroomRetroFutura;

impl RetroFuturaCatalog for CleanroomRetroFutura {
    type Origin = CleanroomOrigin;

    const ORIGIN: CleanroomOrigin = CleanroomOrigin;
    const SOURCE_NAME: &'static str = "cleanroom source";
    const GROUP: &'static str = "com.cleanroommc";
    const ARTIFACT: &'static str = "cleanroom";
    const GAME_VERSION: &'static str = CLEANROOM_GAME_VERSION;
    const RELEASE_DESCRIPTION: &'static str = CLEANROOM_RELEASE_DESCRIPTION;
    const IDENTITY: ProfiledDriverIdentity = CLEANROOM_IDENTITY;
    const FAMILY_NAME: &'static str = "cleanroom";
    const LOADER_VERSION_PREFIX: &'static str = "cleanroom-";
}

pub type CleanroomSpec = RetroFuturaMaven<CleanroomRetroFutura>;
pub type CleanroomEndpoints = RetroFuturaEndpoints<CleanroomRetroFutura>;
pub type CleanroomSource = RetroFuturaSource<CleanroomRetroFutura>;
pub type CleanroomDriverSpec = RetroFuturaFamily<CleanroomRetroFutura>;
pub type CleanroomDriver = RetroFuturaDriver<CleanroomRetroFutura>;
pub type CleanroomCatalog = RetroFuturaReleaseCatalog<CleanroomRetroFutura>;
