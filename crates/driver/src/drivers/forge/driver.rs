use crate::{
    descriptors::FORGE_DRIVER,
    drivers::forge::prepared::ForgeFamily,
    families::installer::{
        InstallerFamilyDriver, InstallerFamilyDriverSpec, LaunchedInstallerFamilyVersion,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
};

pub type ForgeDriver = InstallerFamilyDriver<ForgeFamily>;
pub type LaunchedForgeVersion<L, VL> = LaunchedInstallerFamilyVersion<ForgeFamily, L, VL>;

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
