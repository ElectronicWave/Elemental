use anyhow::Result;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    drivers::forge::source::{ForgeEndpoints, ForgeSource, parse_installer_version},
    families::installer::{
        InstallerFamily, InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
        PreparedInstallerFamilyLaunchVersion, PreparedInstallerFamilyVersion,
        ResolvedInstallerFamilyLaunchVersion, ResolvedInstallerFamilyMetadata,
        ResolvedInstallerFamilyVersion, profile_game_and_raw_loader_version,
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

    fn profile_identity(install_profile: &ForgeInstallerProfile) -> Result<(String, String)> {
        let (game_version, raw_version) =
            profile_game_and_raw_loader_version(install_profile, Self::FAMILY_NAME, "forge")?;
        let (_, loader_version) = parse_installer_version(&raw_version)?;
        Ok((game_version, loader_version))
    }
}
