use std::path::{Path, PathBuf};

use elemental_core::{launcher::command::LaunchCommand, runtime::distribution::Distribution};
use elemental_driver::{
    driver::InstalledDriver,
    drivers::{
        cleanroom::driver::PreparedCleanroomVersion, fabric::prepared::PreparedFabricVersion,
        forge::prepared::PreparedForgeVersion, liteloader::prepared::PreparedLiteLoaderVersion,
        neoforge::prepared::PreparedNeoForgeVersion, quilt::prepared::PreparedQuiltVersion,
        rift::prepared::PreparedRiftVersion, vanilla::prepared::PreparedVanillaVersion,
    },
    families::version_json::{
        BaseInstanceLayout, BaseRootLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
};

use crate::spec::DriverSpec;

pub struct LaunchCommandResult {
    pub runtime: Distribution,
    pub command: LaunchCommand,
}

pub struct LaunchedInstance {
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub instance_name: String,
    pub instance_root: PathBuf,
    pub driver: InstalledDriver,
}

pub struct PreparedInstance<L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(crate) driver: DriverSpec,
    pub(crate) inner: PreparedInstanceKind<L, VL>,
}

pub(crate) enum PreparedInstanceKind<L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    Vanilla(PreparedVanillaVersion<L, VL>),
    FabricLike(PreparedFabricVersion<L, VL>),
    Quilt(PreparedQuiltVersion<L, VL>),
    LiteLoader(PreparedLiteLoaderVersion<L, VL>),
    Rift(PreparedRiftVersion<L, VL>),
    Forge(PreparedForgeVersion<L, VL>),
    Cleanroom(PreparedCleanroomVersion<L, VL>),
    NeoForge(PreparedNeoForgeVersion<L, VL>),
}

impl<L, VL> PreparedInstance<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(crate) fn new(driver: DriverSpec, inner: PreparedInstanceKind<L, VL>) -> Self {
        Self { driver, inner }
    }

    pub fn driver(&self) -> &DriverSpec {
        &self.driver
    }

    pub fn instance_root(&self) -> &Path {
        match &self.inner {
            PreparedInstanceKind::Vanilla(prepared) => {
                prepared.resolved_version.version.path.as_path()
            }
            PreparedInstanceKind::FabricLike(prepared) => {
                prepared.resolved_version.version.path.as_path()
            }
            PreparedInstanceKind::Quilt(prepared) => {
                prepared.resolved_version.version.path.as_path()
            }
            PreparedInstanceKind::LiteLoader(prepared) => {
                prepared.resolved_version.version.path.as_path()
            }
            PreparedInstanceKind::Rift(prepared) => {
                prepared.resolved_version.version.path.as_path()
            }
            PreparedInstanceKind::Forge(prepared) => {
                prepared.resolved_version.instance.path.as_path()
            }
            PreparedInstanceKind::Cleanroom(prepared) => {
                prepared.resolved_version.instance.path.as_path()
            }
            PreparedInstanceKind::NeoForge(prepared) => {
                prepared.resolved_version.instance.path.as_path()
            }
        }
    }
}
