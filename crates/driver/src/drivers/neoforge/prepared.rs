use crate::{
    drivers::neoforge::source::{NeoForgeEndpoints, NeoForgeSource},
    families::installer::{
        InstallerFamily, InstallerFamilyInstallStatus, InstallerFamilyRemoteResolver,
        PreparedInstallerFamilyLaunchVersion, PreparedInstallerFamilyVersion,
        ResolvedInstallerFamilyLaunchVersion, ResolvedInstallerFamilyMetadata,
        ResolvedInstallerFamilyVersion,
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
}
