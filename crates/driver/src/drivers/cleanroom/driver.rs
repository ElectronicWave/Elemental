use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::{
    driver::DriverDescriptor,
    drivers::cleanroom::source::{CleanroomEndpoints, CleanroomSource},
    families::installer::{
        InstallerFamily, InstallerFamilyDriver, InstallerFamilyDriverSpec,
        InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
        LaunchedInstallerFamilyVersion, PreparedInstallerFamilyLaunchVersion,
        PreparedInstallerFamilyVersion, ResolvedInstallerFamilyLaunchVersion,
        ResolvedInstallerFamilyMetadata, ResolvedInstallerFamilyVersion,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CleanroomDriverSpec;

pub type CleanroomDriver = InstallerFamilyDriver<CleanroomDriverSpec>;
pub type CleanroomInstallStatus = InstallerFamilyInstallStatus;
pub type CleanroomRemoteResolver = InstallerFamilyRemoteResolver<CleanroomDriverSpec>;
pub type ResolvedCleanroomMetadata = ResolvedInstallerFamilyMetadata<CleanroomDriverSpec>;
pub type ResolvedCleanroomLaunchVersion<L, VL> =
    ResolvedInstallerFamilyLaunchVersion<CleanroomDriverSpec, L, VL>;
pub type PreparedCleanroomLaunchVersion<L, VL> =
    PreparedInstallerFamilyLaunchVersion<CleanroomDriverSpec, L, VL>;
pub type ResolvedCleanroomVersion<L, VL> =
    ResolvedInstallerFamilyVersion<CleanroomDriverSpec, L, VL>;
pub type PreparedCleanroomVersion<L, VL> =
    PreparedInstallerFamilyVersion<CleanroomDriverSpec, L, VL>;
pub type LaunchedCleanroomVersion<L, VL> =
    LaunchedInstallerFamilyVersion<CleanroomDriverSpec, L, VL>;

impl InstallerFamily for CleanroomDriverSpec {
    type Source = CleanroomSource;
    type Endpoints = CleanroomEndpoints;

    const FAMILY_NAME: &'static str = "cleanroom";

    fn merge_libraries(
        base_libraries: Vec<PistonMetaLibraries>,
        embedded_libraries: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        let filtered_base_libraries = base_libraries
            .into_iter()
            .filter(|library| !is_legacy_cleanroom_runtime_library(library))
            .collect::<Vec<PistonMetaLibraries>>();

        crate::families::installer::merge_libraries_prefer_embedded(
            filtered_base_libraries,
            embedded_libraries,
        )
    }
}

impl InstallerFamilyDriverSpec for CleanroomDriverSpec {
    const DRIVER: DriverDescriptor = DriverDescriptor {
        id: "cleanroom",
        name: "Cleanroom",
    };

    const INSPECT_PREFIXES: &'static [&'static str] = &["com.cleanroommc:cleanroom:"];
}

fn is_legacy_cleanroom_runtime_library(library: &PistonMetaLibraries) -> bool {
    matches_legacy_cleanroom_runtime_prefix(library.name.as_str())
}

fn matches_legacy_cleanroom_runtime_prefix(library_name: &str) -> bool {
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
