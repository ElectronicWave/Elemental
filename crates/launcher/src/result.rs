use std::path::{Path, PathBuf};

use elemental_core::{launcher::command::LaunchCommand, runtime::distribution::Distribution};
use elemental_driver::{
    driver::{DriverDescriptor, InstalledDriver},
    families::{
        installer::PreparedInstallerVersion,
        version_json::{
            BaseInstanceLayout, BaseRootLayout, PreparedVersionJsonInstance,
            VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
    },
};

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
    pub(crate) driver: DriverDescriptor,
    pub(crate) inner: PreparedInstanceKind<L, VL>,
}

pub(crate) enum PreparedInstanceKind<L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    VersionJson(Box<PreparedVersionJsonInstance<L, VL>>),
    Installer(Box<PreparedInstallerVersion<L, VL>>),
}

impl<L, VL> PreparedInstance<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(crate) fn new(driver: DriverDescriptor, inner: PreparedInstanceKind<L, VL>) -> Self {
        Self { driver, inner }
    }

    pub fn driver(&self) -> &DriverDescriptor {
        &self.driver
    }

    pub fn instance_root(&self) -> &Path {
        match &self.inner {
            PreparedInstanceKind::VersionJson(prepared) => prepared.instance_root(),
            PreparedInstanceKind::Installer(prepared) => prepared.instance_root(),
        }
    }
}
