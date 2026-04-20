use std::{
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use elemental_core::{
    consts::PLATFORM_NATIVES_DIR_NAME,
    jar::JarFile,
    storage::{
        Storage,
        layout::{Layout, Layoutable},
    },
};
use tokio::fs::create_dir_all;

use super::resource::Resource;
use crate::{
    driver::Driver,
    drivers::version_json::{
        PistonMetaAssetIndexObjects, PistonMetaData, extensions::PistonMetaLibrariesExt,
        rules::MojangRuleContext,
    },
    inspect::{InstalledInstance, inspect_instance},
};

#[async_trait]
pub trait VersionJsonGameStorageExt {
    type Layout: Layout<Resource = Resource>;

    fn instances_root_path(&self) -> Result<PathBuf>;
    fn instance_root_path(&self, name: impl AsRef<str>) -> Result<PathBuf>;
    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: Layout + Send;
    fn instance_exists(&self, name: impl AsRef<str>) -> Result<bool>;
    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: Layout;
    fn instance_names(&self) -> Result<Vec<String>>;
    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: Layout + Clone;
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
    L: Layout<Resource = Resource>,
{
    type Layout = L;

    fn instances_root_path(&self) -> Result<PathBuf> {
        self.get_resource(Resource::Versions)
            .context("versions resource is not available")
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
        VL: Layout + Send,
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
        VL: Layout,
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
        VL: Layout + Clone,
    {
        self.instance_names()?
            .into_iter()
            .map(|name| self.instance(name, version_layout.clone()))
            .collect()
    }

    fn asset_index_path(&self, id: impl AsRef<str>) -> Result<PathBuf> {
        self.get_resource(Resource::AssetsIndexes)
            .map(|path| path.join(format!("{}.json", id.as_ref())))
            .context("asset indexes resource is not available")
    }

    fn asset_object_path(&self, hash: impl AsRef<str>) -> Result<PathBuf> {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).context("asset hash is too short")?;

        self.get_resource(Resource::AssetsObjects)
            .map(|path| path.join(prefix).join(hash))
            .context("asset objects resource is not available")
    }

    fn library_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        self.get_resource(Resource::Libraries)
            .map(|root| root.join(path))
            .context("libraries resource is not available")
    }

    fn logging_config_path(&self, file_id: impl AsRef<str>) -> Result<PathBuf> {
        self.get_resource(Resource::AssetsLogConfigs)
            .map(|root| root.join(file_id.as_ref()))
            .context("logging configs resource is not available")
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
    type GameLayout: Layout<Resource = Resource>;
    type VersionLayout: Layout;

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
    L: Layout<Resource = Resource>,
    VL: Layout,
{
    type GameLayout = L;
    type VersionLayout = VL;

    fn metadata_path(&self) -> Result<PathBuf> {
        Ok(self.path.join(format!(
            "{}.json",
            self.name().context("get version name failed")?
        )))
    }

    fn metadata(&self) -> Result<PistonMetaData> {
        Ok(serde_json::from_reader(File::open(self.metadata_path()?)?)?)
    }

    fn jar_path(&self) -> Result<PathBuf> {
        Ok(self.path.join(format!(
            "{}.jar",
            self.name().context("get version name failed")?
        )))
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
        self.path.join(PLATFORM_NATIVES_DIR_NAME)
    }

    fn natives_marker_path(&self) -> PathBuf {
        self.path.join(".elemental-natives-ready")
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
        let rule_context = MojangRuleContext::current();
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
    L: Layout<Resource = Resource> + Clone,
    VL: Layout + Clone,
{
    let mut instances = Vec::new();
    for instance in storage.instances(version_layout)? {
        if let Some(installed) = inspect_instance(instance, drivers).await? {
            instances.push(installed);
        }
    }

    Ok(instances)
}
