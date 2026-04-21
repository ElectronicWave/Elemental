use anyhow::Result;
use elemental_core::storage::Storage;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::neoforge::source::{NeoForgeEndpoints, NeoForgeSource},
    families::{
        installer::{
            InstallerArtifact, InstallerFamily, InstallerFamilyInstallStatus,
            InstallerFamilyRemoteResolver, PreparedInstallerFamilyLaunchVersion,
            PreparedInstallerFamilyVersion, ResolvedInstallerFamilyLaunchVersion,
            ResolvedInstallerFamilyMetadata, ResolvedInstallerFamilyVersion,
            normalize_library_urls, profile_game_and_raw_loader_version,
        },
        version_json::VersionJsonRootLayout,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NeoForgeFamily;

pub type NeoForgeInstallStatus = InstallerFamilyInstallStatus;
pub type NeoForgeRemoteResolver = InstallerFamilyRemoteResolver<NeoForgeFamily>;
pub type ResolvedNeoForgeMetadata = ResolvedInstallerFamilyMetadata<NeoForgeFamily>;
pub type ResolvedNeoForgeLaunchVersion<L, VL> =
    ResolvedInstallerFamilyLaunchVersion<NeoForgeFamily, L, VL>;
pub type PreparedNeoForgeLaunchVersion<L, VL> =
    PreparedInstallerFamilyLaunchVersion<NeoForgeFamily, L, VL>;
pub type ResolvedNeoForgeVersion<L, VL> = ResolvedInstallerFamilyVersion<NeoForgeFamily, L, VL>;
pub type PreparedNeoForgeVersion<L, VL> = PreparedInstallerFamilyVersion<NeoForgeFamily, L, VL>;

impl InstallerFamily for NeoForgeFamily {
    type Source = NeoForgeSource;
    type Endpoints = NeoForgeEndpoints;

    const FAMILY_NAME: &'static str = "neoforge";

    fn source_endpoints(source: &Self::Source) -> &Self::Endpoints {
        source.endpoints()
    }

    fn installer_artifact<L>(
        source: &Self::Source,
        game_storage: &Storage<L>,
        game_version: &str,
        loader_version: &str,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout,
    {
        source.installer_artifact(game_storage, game_version, loader_version)
    }

    fn profile_identity(install_profile: &ForgeInstallerProfile) -> Result<(String, String)> {
        profile_game_and_raw_loader_version(install_profile, Self::FAMILY_NAME, Self::FAMILY_NAME)
    }

    fn normalize_libraries(
        endpoints: &Self::Endpoints,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>> {
        normalize_library_urls(libraries, |artifact_path| {
            endpoints.maven_artifact_url(artifact_path)
        })
    }

    fn rewrite_upstream(endpoints: &Self::Endpoints, raw_url: &str) -> Result<String> {
        endpoints.rewrite_upstream(raw_url)
    }

    fn default_artifact_url(endpoints: &Self::Endpoints, artifact_path: &str) -> Result<String> {
        endpoints.maven_artifact_url(artifact_path)
    }
}
