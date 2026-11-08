use std::{
    fs,
    path::{Path, PathBuf},
};

use elemental_core::{
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layoutable},
};
use elemental_driver::{
    driver::{DriverDescriptor, InstalledDriver},
    families::version_json::{
        BaseInstanceLayout, BaseRootLayout, PreparedVersionJsonInstance, VersionJsonInstanceLayout,
        VersionJsonInstanceResource, VersionJsonRootLayout,
    },
};

use crate::error::Result;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedInstanceMetadata {
    pub id: String,
    pub main_class: String,
    pub inherits_from: Option<String>,
    pub asset_index: String,
    pub release_type: String,
    pub required_java_major_version: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedInstanceInfo {
    pub metadata: PreparedInstanceMetadata,
    pub version_root: PathBuf,
    pub metadata_path: PathBuf,
    pub version_jar: PathBuf,
    pub natives_directory: PathBuf,
    pub natives_files: Vec<PathBuf>,
    pub classpath: Vec<PathBuf>,
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

    pub fn info(&self) -> Result<PreparedInstanceInfo> {
        let version = &self.launch_version.version;
        let metadata_path = version.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        let version_jar = version.try_get_resource(VersionJsonInstanceResource::Jar)?;
        let natives_directory = version.try_get_resource(VersionJsonInstanceResource::Natives)?;
        let mut natives_files = Vec::new();
        collect_files(&natives_directory, &mut natives_files)?;
        natives_files.sort();

        Ok(PreparedInstanceInfo {
            metadata: PreparedInstanceMetadata {
                id: self.launch_version.metadata.id.clone(),
                main_class: self.launch_version.metadata.main_class.clone(),
                inherits_from: self.launch_version.metadata.inherits_from.clone(),
                asset_index: self.launch_version.metadata.asset_index.id.clone(),
                release_type: self.launch_version.metadata.release_type.clone(),
                required_java_major_version: self.launch_version.required_java_major_version(),
            },
            version_root: version.path.clone(),
            metadata_path,
            version_jar,
            natives_directory,
            natives_files,
            classpath: self.launch_version.classpath()?,
        })
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }

    Ok(())
}
