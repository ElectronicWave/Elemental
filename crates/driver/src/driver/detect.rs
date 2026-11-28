use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layoutable;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{
        GameStorage, InstanceStorage, SharedInstanceLayout, VersionJsonGameStorageExt,
        VersionJsonInstanceResource, patch::baseline_path,
    },
    inspect::InstalledDriver,
};

use super::Driver;

#[derive(Debug, Clone)]
pub struct InstanceProbe {
    pub metadata: Option<PistonMetaData>,
}

impl InstanceProbe {
    pub async fn collect(storage: &InstanceStorage) -> Result<Self> {
        let baseline = baseline_path(storage)?;
        if tokio::fs::metadata(&baseline)
            .await
            .is_ok_and(|metadata| metadata.is_file())
        {
            let bytes = tokio::fs::read(&baseline).await?;
            return Ok(Self {
                metadata: Some(serde_json::from_slice(&bytes)?),
            });
        }

        Self::collect_raw(storage).await
    }

    pub async fn collect_raw(storage: &InstanceStorage) -> Result<Self> {
        let preferred = storage.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        let metadata_path = match tokio::fs::metadata(&preferred).await {
            Ok(metadata) if metadata.is_file() => Some(preferred),
            Ok(_) => Self::single_json_metadata(storage.root_path()).await?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::single_json_metadata(storage.root_path()).await?
            }
            Err(error) => return Err(error.into()),
        };

        let metadata = match metadata_path {
            Some(path) => {
                let bytes = tokio::fs::read(path).await?;
                Some(serde_json::from_slice(&bytes)?)
            }
            None => None,
        };

        Ok(Self { metadata })
    }

    async fn single_json_metadata(directory: &Path) -> Result<Option<PathBuf>> {
        let mut entries = match tokio::fs::read_dir(directory).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        let mut candidates = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let is_json = path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
            if !entry.file_type().await?.is_file() || !is_json {
                continue;
            }
            candidates.push(path);
        }

        Ok(candidates.pop().filter(|_| candidates.is_empty()))
    }
}

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
            let instance_name = instance.display_name();
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

pub async fn inspect_instances(
    storage: &GameStorage,
    version_layout: SharedInstanceLayout,
    drivers: &[&dyn Driver],
) -> Result<Vec<std::result::Result<InstalledInstance, InstanceDetectionFailure>>> {
    InstalledInstance::detect_all(storage, version_layout, drivers).await
}
