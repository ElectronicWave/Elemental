use std::path::{Path, PathBuf};

use elemental_core::{
    launcher::process::ProcessLogLine, runtime::distribution::Distribution, storage::Storage,
};
use elemental_driver::{
    driver::{DriverDescriptor, InstalledDriver},
    families::version_json::{
        BaseInstanceLayout, BaseRootLayout, PreparedVersionJsonInstance, VersionJsonInstanceLayout,
        VersionJsonRootLayout,
    },
};

pub struct LaunchedInstance {
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

pub struct LoggedLaunchedInstance {
    pub runtime: Distribution,
    pub child: tokio::process::Child,
    pub lines: tokio::sync::mpsc::UnboundedReceiver<ProcessLogLine>,
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
    pub(crate) launch_version: PreparedVersionJsonInstance<L, VL>,
}

impl<L, VL> PreparedInstance<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub(crate) fn new(
        driver: DriverDescriptor,
        launch_version: PreparedVersionJsonInstance<L, VL>,
    ) -> Self {
        Self {
            driver,
            launch_version,
        }
    }

    pub fn driver(&self) -> &DriverDescriptor {
        &self.driver
    }

    pub fn instance_root(&self) -> &Path {
        self.launch_version.instance_root()
    }

    pub fn version(&self) -> &Storage<VL, Storage<L>> {
        &self.launch_version.version
    }
}
