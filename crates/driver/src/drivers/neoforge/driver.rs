use crate::{
    descriptors::NEOFORGE_DRIVER,
    drivers::neoforge::prepared::NeoForgeFamily,
    families::installer::{
        InstallerFamilyDriver, InstallerFamilyDriverSpec, LaunchedInstallerFamilyVersion,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
};

pub type NeoForgeDriver = InstallerFamilyDriver<NeoForgeFamily>;
pub type LaunchedNeoForgeVersion<L, VL> = LaunchedInstallerFamilyVersion<NeoForgeFamily, L, VL>;

const NEOFORGE_MARKERS: LibraryPrefixSet = LibraryPrefixSet::new(&[
    "net.neoforged:neoforge:",
    "net.neoforged:forge:",
    "net.neoforged:fmlloader:",
    "net.neoforged.fancymodloader:",
]);
const NEOFORGE_VERSION_LIBRARIES: LibraryPrefixSet = LibraryPrefixSet::new(&[
    "net.neoforged:neoforge:",
    "net.neoforged:forge:",
    "net.neoforged:fmlloader:",
]);
const NEOFORGE_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    NEOFORGE_DRIVER,
    NEOFORGE_MARKERS,
    NEOFORGE_VERSION_LIBRARIES,
    ProfileIdPattern::loader_prefixed("neoforge-"),
);

impl InstallerFamilyDriverSpec for NeoForgeFamily {
    const IDENTITY: ProfiledDriverIdentity = NEOFORGE_IDENTITY;
}
