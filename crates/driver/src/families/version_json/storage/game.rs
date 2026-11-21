mod helpers;
mod storage_impl;

use std::path::{Path, PathBuf};

use elemental_core::storage::Storage;

use crate::{
    error::Result,
    families::version_json::layout::{
        GameStorage, SharedInstanceLayout, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
    inspect::{InstalledInstance, InstanceDetectionFailure},
};

#[async_trait::async_trait]
pub trait VersionJsonGameStorageExt {
    type Layout: VersionJsonRootLayout;

    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send;

    fn instance_exists<VL>(&self, name: impl AsRef<str>, version_layout: &VL) -> Result<bool>
    where
        VL: VersionJsonInstanceLayout;

    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout;

    fn instance_names<VL>(&self, version_layout: &VL) -> Result<Vec<String>>
    where
        VL: VersionJsonInstanceLayout;

    fn instance_root(&self, name: impl AsRef<str>) -> Result<PathBuf>;

    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone;
}

pub async fn inspect_instances(
    storage: &GameStorage,
    version_layout: SharedInstanceLayout,
    drivers: &[&dyn crate::driver::Driver],
) -> Result<Vec<std::result::Result<InstalledInstance, InstanceDetectionFailure>>> {
    InstalledInstance::detect_all(storage, version_layout, drivers).await
}

pub(super) fn instance_names_in<L, VL>(
    storage: &Storage<L>,
    root: &Path,
    version_layout: &VL,
) -> Result<Vec<String>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let mut instances = Vec::new();
    for entry in root.read_dir()? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        instances.extend(
            storage
                .instance_exists(&name, version_layout)?
                .then_some(name),
        );
    }
    Ok(instances)
}
