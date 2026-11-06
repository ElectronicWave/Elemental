use std::path::Path;

use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::version_json::{
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::{VersionJsonInstanceResource, VersionJsonRootResource},
    },
    inspect::InstalledInstance,
};

#[async_trait(?Send)]
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
        let root = self.try_get_resource(VersionJsonRootResource::Versions(Some(name)))?;
        create_dir_all(&root).await?;
        Ok(Storage::with_parent(root, self.clone(), version_layout))
    }

    fn instance_exists<VL>(&self, name: impl AsRef<str>, version_layout: &VL) -> Result<bool>
    where
        VL: VersionJsonInstanceLayout,
    {
        let root = self.try_get_resource(VersionJsonRootResource::Versions(Some(
            name.as_ref().to_owned(),
        )))?;
        instance_ready(&root, version_layout)
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
        let root = self.try_get_resource(VersionJsonRootResource::Versions(Some(name.clone())))?;
        if !instance_ready(&root, &version_layout)? {
            return Err(Error::InstanceNotFound { name });
        }
        Ok(Storage::with_parent(root, self.clone(), version_layout))
    }

    fn instance_names<VL>(&self, version_layout: &VL) -> Result<Vec<String>>
    where
        VL: VersionJsonInstanceLayout,
    {
        let root = self.try_get_resource(VersionJsonRootResource::Versions(None))?;
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

fn instance_ready<VL>(root: &Path, version_layout: &VL) -> Result<bool>
where
    VL: VersionJsonInstanceLayout,
{
    Ok(version_layout
        .try_get_resource(root, VersionJsonInstanceResource::Metadata)?
        .exists()
        && version_layout
            .try_get_resource(root, VersionJsonInstanceResource::Jar)?
            .exists())
}

fn instance_names_in<L, VL>(
    storage: &Storage<L>,
    root: &std::path::Path,
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
        if storage.instance_exists(&name, version_layout)? {
            instances.push(name);
        }
    }
    Ok(instances)
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
