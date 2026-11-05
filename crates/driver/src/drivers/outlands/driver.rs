use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::{
    descriptors::OUTLANDS_DRIVER,
    drivers::outlands::source::{OutlandsEndpoints, OutlandsSource},
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerFamilyDriverSpec,
        InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
        LaunchedInstallerFamilyVersion, PreparedInstallerFamilyLaunchVersion,
        PreparedInstallerFamilyVersion, ResolvedInstallerFamilyLaunchVersion,
        ResolvedInstallerFamilyMetadata, ResolvedInstallerFamilyVersion,
    },
    inspect::{LibraryPrefixSet, ProfileIdPattern, ProfiledDriverIdentity},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutlandsDriverSpec;

pub type OutlandsDriver = InstallerFamilyDriver<OutlandsDriverSpec>;
pub type OutlandsInstallStatus = InstallerFamilyInstallStatus;
pub type OutlandsRemoteResolver = InstallerFamilyRemoteResolver<OutlandsDriverSpec>;
pub type ResolvedOutlandsMetadata = ResolvedInstallerFamilyMetadata<OutlandsDriverSpec>;
pub type ResolvedOutlandsLaunchVersion<L, VL> =
ResolvedInstallerFamilyLaunchVersion<OutlandsDriverSpec, L, VL>;
pub type PreparedOutlandsLaunchVersion<L, VL> =
PreparedInstallerFamilyLaunchVersion<OutlandsDriverSpec, L, VL>;
pub type ResolvedOutlandsVersion<L, VL> =
ResolvedInstallerFamilyVersion<OutlandsDriverSpec, L, VL>;
pub type PreparedOutlandsVersion<L, VL> =
PreparedInstallerFamilyVersion<OutlandsDriverSpec, L, VL>;
pub type LaunchedOutlandsVersion<L, VL> =
LaunchedInstallerFamilyVersion<OutlandsDriverSpec, L, VL>;

impl InstallerFamily for OutlandsDriverSpec {
    type Source = OutlandsSource;
    type Endpoints = OutlandsEndpoints;

    const FAMILY_NAME: &'static str = "outlands";
    const LOADER_VERSION_PREFIX: &'static str = "outlands-";

    fn merge_libraries(
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        let filtered_base_libraries = base_libraries
            .into_iter()
            .filter(|library| !is_legacy_outlands_runtime_library(library))
            .collect::<Vec<PistonMetaLibraries>>();

        crate::families::installer::merge_libraries_prefer_embedded(
            filtered_base_libraries,
            embedded_libraries,
        )
    }
}

const OUTLANDS_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["top.outlands:outlands:"]);
const OUTLANDS_IDENTITY: ProfiledDriverIdentity = ProfiledDriverIdentity::new(
    OUTLANDS_DRIVER,
    OUTLANDS_LIBRARIES,
    OUTLANDS_LIBRARIES,
    ProfileIdPattern::new("-outlands-"),
);

impl InstallerFamilyDriverSpec for OutlandsDriverSpec {
    const IDENTITY: ProfiledDriverIdentity = OUTLANDS_IDENTITY;
}

fn is_legacy_outlands_runtime_library(library: &PistonMetaLibraries) -> bool {
    matches_legacy_outlands_runtime_prefix(library.name.as_str())
}

fn matches_legacy_outlands_runtime_prefix(library_name: &str) -> bool {
    library_name.starts_with("org.lwjgl.lwjgl:lwjgl:")
        || library_name.starts_with("org.lwjgl.lwjgl:lwjgl_util:")
        || library_name.starts_with("org.lwjgl.lwjgl:lwjgl-platform:")
        || library_name.starts_with("net.java.jinput:jinput:")
        || library_name.starts_with("net.java.jinput:jinput-platform:")
        || library_name.starts_with("net.java.jutils:jutils:")
        || library_name.starts_with("net.java.dev.jna:platform:")
        || library_name.starts_with("oshi-project:oshi-core:")
        || library_name.starts_with("com.ibm.icu:icu4j-core-mojang:")
}
