use std::{
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use elemental_core::{jar::JarFile, storage::Storage};
use tokio::fs::create_dir_all;

use super::layout::{VersionJsonInstanceLayout, VersionJsonRootLayout};
use crate::{
    driver::Driver,
    drivers::version_json::{
        PistonMetaAssetIndexObjects, PistonMetaData, extensions::PistonMetaLibrariesExt,
        rules::VersionJsonRuleContext,
    },
    inspect::{InstalledInstance, inspect_instance},
};

#[async_trait]
pub trait VersionJsonGameStorageExt {
    type Layout: VersionJsonRootLayout;

    fn instances_root_path(&self) -> Result<PathBuf>;
    fn instance_root_path(&self, name: impl AsRef<str>) -> Result<PathBuf>;
    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send;
    fn instance_exists(&self, name: impl AsRef<str>) -> Result<bool>;
    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout;
    fn instance_names(&self) -> Result<Vec<String>>;
    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone;
    fn asset_index_path(&self, id: impl AsRef<str>) -> Result<PathBuf>;
    fn asset_object_path(&self, hash: impl AsRef<str>) -> Result<PathBuf>;
    fn library_path(&self, path: impl AsRef<Path>) -> Result<PathBuf>;
    fn logging_config_path(&self, file_id: impl AsRef<str>) -> Result<PathBuf>;
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()>;
    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects>;
}

#[async_trait]
impl<L> VersionJsonGameStorageExt for Storage<L>
where
    L: VersionJsonRootLayout,
{
    type Layout = L;

    fn instances_root_path(&self) -> Result<PathBuf> {
        Ok(self.layout.instances_root_path(&self.path))
    }

    fn instance_root_path(&self, name: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.instances_root_path()?.join(name.as_ref()))
    }

    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send,
    {
        let instances_root = self.instances_root_path()?;
        create_dir_all(&instances_root).await?;

        let instance_root = instances_root.join(&name);
        create_dir_all(&instance_root).await?;

        Ok(Storage::with_parent(
            instance_root,
            self.clone(),
            version_layout,
        ))
    }

    fn instance_exists(&self, name: impl AsRef<str>) -> Result<bool> {
        let name = name.as_ref();
        let instance_root = self.instance_root_path(name)?;

        Ok(instance_root.join(format!("{name}.jar")).exists()
            && instance_root.join(format!("{name}.json")).exists())
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
        if !self.instance_exists(&name)? {
            return Err(anyhow!("can't find a valid instance named '{name}'"));
        }

        Ok(Storage::with_parent(
            self.instance_root_path(&name)?,
            self.clone(),
            version_layout,
        ))
    }

    fn instance_names(&self) -> Result<Vec<String>> {
        let instances_root = self.instances_root_path()?;
        if !instances_root.exists() {
            return Ok(Vec::new());
        }

        let mut instances = Vec::new();
        for entry in instances_root.read_dir()? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if self.instance_exists(&name)? {
                instances.push(name);
            }
        }

        Ok(instances)
    }

    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        self.instance_names()?
            .into_iter()
            .map(|name| self.instance(name, version_layout.clone()))
            .collect()
    }

    fn asset_index_path(&self, id: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.layout.asset_index_path(&self.path, id.as_ref()))
    }

    fn asset_object_path(&self, hash: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.layout.asset_object_path(&self.path, hash.as_ref()))
    }

    fn library_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        Ok(self.layout.library_path(&self.path, path.as_ref()))
    }

    fn logging_config_path(&self, file_id: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self
            .layout
            .logging_config_path(&self.path, file_id.as_ref()))
    }

    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()> {
        let path = self.asset_index_path(id)?;
        let parent = path
            .parent()
            .context("asset index path has no parent directory")?;
        create_dir_all(parent).await?;
        tokio::fs::write(path, serde_json::to_vec(objects)?).await?;
        Ok(())
    }

    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects> {
        Ok(serde_json::from_reader(File::open(
            self.asset_index_path(id)?,
        )?)?)
    }
}

#[async_trait]
pub trait VersionJsonVersionStorageExt {
    type GameLayout: VersionJsonRootLayout;
    type VersionLayout: VersionJsonInstanceLayout;

    fn metadata_path(&self) -> Result<PathBuf>;
    fn metadata(&self) -> Result<PistonMetaData>;
    fn jar_path(&self) -> Result<PathBuf>;
    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()>;
    fn platform_natives_path(&self) -> PathBuf;
    fn natives_marker_path(&self) -> PathBuf;
    fn natives_are_extracted(&self) -> bool;
    async fn ensure_platform_natives_path(&self) -> Result<PathBuf>;
    fn extract_natives(&self) -> Result<()>;
}

#[async_trait]
impl<L, VL> VersionJsonVersionStorageExt for Storage<VL, Storage<L>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    type GameLayout = L;
    type VersionLayout = VL;

    fn metadata_path(&self) -> Result<PathBuf> {
        let name = self.name().context("get version name failed")?;
        Ok(self.layout.metadata_path(&self.path, &name))
    }

    fn metadata(&self) -> Result<PistonMetaData> {
        Ok(serde_json::from_reader(File::open(self.metadata_path()?)?)?)
    }

    fn jar_path(&self) -> Result<PathBuf> {
        let name = self.name().context("get version name failed")?;
        Ok(self.layout.jar_path(&self.path, &name))
    }

    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()> {
        self.ensure_root().await?;
        tokio::fs::write(self.metadata_path()?, serde_json::to_vec(metadata)?).await?;
        let marker = self.natives_marker_path();
        if marker.exists() {
            tokio::fs::remove_file(marker).await?;
        }
        Ok(())
    }

    fn platform_natives_path(&self) -> PathBuf {
        self.layout.platform_natives_path(&self.path)
    }

    fn natives_marker_path(&self) -> PathBuf {
        self.layout.natives_marker_path(&self.path)
    }

    fn natives_are_extracted(&self) -> bool {
        self.natives_marker_path().exists()
    }

    async fn ensure_platform_natives_path(&self) -> Result<PathBuf> {
        let path = self.platform_natives_path();
        create_dir_all(&path).await?;
        Ok(path)
    }

    fn extract_natives(&self) -> Result<()> {
        let metadata = self.metadata()?;
        let destination = self.platform_natives_path();
        let rule_context = VersionJsonRuleContext::current();
        std::fs::create_dir_all(&destination)?;

        for library in metadata.libraries {
            if !library.is_allowed(&rule_context) {
                continue;
            }

            if let Some(artifact) = library.classifiers_native_artifact(rule_context.platform()) {
                let source = self.parent.library_path(artifact.path.as_str())?;
                JarFile::new(source).extract_blocking(&destination)?;
            }

            if let Some(artifact) = library.native_artifact(rule_context.platform()) {
                let source = self.parent.library_path(artifact.path.as_str())?;
                JarFile::new(source).extract_blocking(&destination)?;
            }
        }

        std::fs::write(self.natives_marker_path(), b"ready")?;

        Ok(())
    }
}

pub async fn inspect_instances<L, VL>(
    storage: &Storage<L>,
    version_layout: VL,
    drivers: &[&dyn Driver<L, VL>],
) -> Result<Vec<InstalledInstance<L, VL>>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut instances = Vec::new();
    for instance in storage.instances(version_layout)? {
        if let Some(installed) = inspect_instance(instance, drivers).await? {
            instances.push(installed);
        }
    }

    Ok(instances)
}
