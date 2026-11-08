use async_trait::async_trait;
use elemental_core::storage::Storage;
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::version_json::layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
};

use super::{
    VersionJsonGameStorageExt,
    helpers::{instance_ready, instance_root, versions_root},
    instance_names_in,
};

#[async_trait(?Send)]
impl<L> VersionJsonGameStorageExt for Storage<L>
where
    L: VersionJsonRootLayout,
{
    type Layout = L;

    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send,
    {
        let root = instance_root(self, name)?;
        create_dir_all(&root).await?;
        Ok(Storage::with_parent(root, self.clone(), version_layout))
    }

    fn instance_exists<VL>(&self, name: impl AsRef<str>, version_layout: &VL) -> Result<bool>
    where
        VL: VersionJsonInstanceLayout,
    {
        instance_ready(&instance_root(self, name)?, version_layout)
    }

    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout,
    {
        let name = name.into();
        let root = instance_root(self, name.clone())?;
        if !instance_ready(&root, &version_layout)? {
            return Err(Error::InstanceNotFound { name });
        }
        Ok(Storage::with_parent(root, self.clone(), version_layout))
    }

    fn instance_names<VL>(&self, version_layout: &VL) -> Result<Vec<String>>
    where
        VL: VersionJsonInstanceLayout,
    {
        let root = versions_root(self)?;
        if !root.exists() {
            return Ok(Vec::new());
        }

        instance_names_in(self, &root, version_layout)
    }

    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        self.instance_names(&version_layout)?
            .into_iter()
            .map(|name| self.instance(name, version_layout.clone()))
            .collect()
    }
}
