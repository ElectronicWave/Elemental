use std::path::PathBuf;

use elemental_core::storage::{Storage, layout::Layout};

use crate::{
    driver::{Driver, InstalledDriver},
    error::Result,
    families::version_json::{
        VersionJsonGameStorageExt, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
};

use super::InstanceProbe;

#[derive(Debug, Clone)]
pub struct InstalledInstance<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub driver: InstalledDriver,
}

#[derive(Debug)]
pub struct InstanceDetectionFailure {
    pub instance_name: String,
    pub path: PathBuf,
    pub error: crate::error::Error,
}

impl<L: Layout + Clone, VL: VersionJsonInstanceLayout + Clone> InstalledInstance<L, VL> {
    pub async fn detect(
        storage: Storage<VL, Storage<L>>,
        drivers: &[&dyn Driver<L, VL>],
    ) -> Result<Option<Self>> {
        let probe = InstanceProbe::collect(&storage)?;
        for driver in drivers {
            if let Some(installed) = driver.inspect(&probe).await? {
                return Ok(Some(Self {
                    storage,
                    driver: installed,
                }));
            }
        }

        Ok(None)
    }
}

impl<L, VL> InstalledInstance<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn detect_all(
        storage: &Storage<L>,
        version_layout: VL,
        drivers: &[&dyn Driver<L, VL>],
    ) -> Result<Vec<std::result::Result<Self, InstanceDetectionFailure>>> {
        let mut detected = Vec::new();

        for instance in storage.instances(version_layout)? {
            let instance_name = instance_name(&instance);
            let path = instance.path.clone();
            let result = match Self::detect(instance, drivers).await {
                Ok(Some(installed)) => Ok(installed),
                Ok(None) => continue,
                Err(error) => Err(InstanceDetectionFailure {
                    instance_name,
                    path,
                    error,
                }),
            };
            detected.push(result);
        }

        Ok(detected)
    }
}

fn instance_name<L, VL>(instance: &Storage<VL, Storage<L>>) -> String
where
    L: Layout,
    VL: Layout,
{
    instance.name().unwrap_or_else(|| {
        instance
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| instance.path.display().to_string())
    })
}
