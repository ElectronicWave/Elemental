use crate::{
    drivers::forge::source::{ForgeEndpoints, ForgeSource},
    families::installer::{
        InstallerFamily, InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
        PreparedInstallerFamilyLaunchVersion, PreparedInstallerFamilyVersion,
        ResolvedInstallerFamilyLaunchVersion, ResolvedInstallerFamilyMetadata,
        ResolvedInstallerFamilyVersion,
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
    const LOADER_VERSION_PREFIX: &'static str = "forge-";
}
