mod helpers;
mod scan;
mod storage_impl;

use elemental_core::storage::Storage;

use crate::{
    error::Result,
    families::version_json::layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    inspect::InstalledInstance,
};

#[async_trait::async_trait(?Send)]
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

    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone;
}

pub async fn inspect_instances<L, VL>(
    storage: &Storage<L>,
    version_layout: VL,
    drivers: &[&dyn crate::driver::Driver<L, VL>],
) -> Result<Vec<InstalledInstance<L, VL>>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    InstalledInstance::detect_all(storage, version_layout, drivers).await
}
