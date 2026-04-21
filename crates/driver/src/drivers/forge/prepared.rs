use anyhow::Result;
use elemental_core::storage::Storage;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::forge::source::{ForgeEndpoints, ForgeSource, parse_installer_version},
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
pub struct ForgeFamily;

pub type ForgeInstallStatus = InstallerFamilyInstallStatus;
pub type ForgeRemoteResolver = InstallerFamilyRemoteResolver<ForgeFamily>;
pub type ResolvedForgeMetadata = ResolvedInstallerFamilyMetadata<ForgeFamily>;
pub type ResolvedForgeLaunchVersion<L, VL> =
    ResolvedInstallerFamilyLaunchVersion<ForgeFamily, L, VL>;
pub type PreparedForgeLaunchVersion<L, VL> =
    PreparedInstallerFamilyLaunchVersion<ForgeFamily, L, VL>;
pub type ResolvedForgeVersion<L, VL> = ResolvedInstallerFamilyVersion<ForgeFamily, L, VL>;
pub type PreparedForgeVersion<L, VL> = PreparedInstallerFamilyVersion<ForgeFamily, L, VL>;

impl InstallerFamily for ForgeFamily {
    type Source = ForgeSource;
    type Endpoints = ForgeEndpoints;

    const FAMILY_NAME: &'static str = "forge";

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
        let (game_version, raw_version) =
            profile_game_and_raw_loader_version(install_profile, Self::FAMILY_NAME, "forge")?;
        let (_, loader_version) = parse_installer_version(&raw_version)?;
        Ok((game_version, loader_version))
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
