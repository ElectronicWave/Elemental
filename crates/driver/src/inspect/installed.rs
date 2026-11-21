use std::path::PathBuf;

use crate::{
    driver::{Driver, InstalledDriver},
    error::Result,
    families::version_json::{
        GameStorage, InstanceStorage, SharedInstanceLayout, VersionJsonGameStorageExt,
    },
};

use super::InstanceProbe;

#[derive(Debug, Clone)]
pub struct InstalledInstance {
    pub storage: InstanceStorage,
    pub driver: InstalledDriver,
}

#[derive(Debug)]
pub struct InstanceDetectionFailure {
    pub instance_name: String,
    pub path: PathBuf,
    pub error: crate::error::Error,
}

impl InstalledInstance {
    pub async fn detect(storage: InstanceStorage, drivers: &[&dyn Driver]) -> Result<Option<Self>> {
        let probe = InstanceProbe::collect(&storage).await?;
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

    pub async fn detect_all(
        storage: &GameStorage,
        version_layout: SharedInstanceLayout,
        drivers: &[&dyn Driver],
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

fn instance_name(instance: &InstanceStorage) -> String {
    instance.name().unwrap_or_else(|| {
        instance
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| instance.path.display().to_string())
    })
}
