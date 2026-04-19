use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use tokio::fs::create_dir_all;

use crate::mojang::PistonMetaAssetIndexObjects;
use crate::storage::{
    layout::{Layout, Layoutable},
    resource::Resource,
    version::VersionStorage,
};

#[derive(Debug, Clone)]
pub struct GameStorage<L: Layout> {
    pub path: PathBuf,
    pub layout: L,
}

impl<L: Layout> GameStorage<L> {
    pub fn new<P: AsRef<Path>>(path: P, layout: L) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            layout,
        }
    }

    pub fn new_ensure_dir<P: AsRef<Path>>(path: P, layout: L) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(Self { path, layout })
    }

    pub fn objectindex(&self, id: impl Display) -> Option<PathBuf> {
        self.layout
            .get_resource(&self.path, Resource::AssetsIndexes)
            .and_then(|path| Some(path.join(format!("{}.json", id))))
    }

    pub fn versions_root_path(&self) -> Result<PathBuf> {
        self.get_resource(Resource::Versions)
            .context("versions resource is not available")
    }

    pub fn version_root_path(&self, name: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.versions_root_path()?.join(name.as_ref()))
    }

    pub fn asset_index_path(&self, id: impl Display) -> Result<PathBuf> {
        self.get_resource(Resource::AssetsIndexes)
            .map(|path| path.join(format!("{}.json", id)))
            .context("asset indexes resource is not available")
    }

    pub fn asset_object_path(&self, hash: impl AsRef<str>) -> Result<PathBuf> {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).context("asset hash is too short")?;
        self.get_resource(Resource::AssetsObjects)
            .map(|path| path.join(prefix).join(hash))
            .context("asset objects resource is not available")
    }

    pub fn library_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        self.get_resource(Resource::Libraries)
            .map(|root| root.join(path))
            .context("libraries resource is not available")
    }

    pub fn logging_config_path(&self, file_id: impl AsRef<str>) -> Result<PathBuf> {
        self.get_resource(Resource::AssetsLogConfigs)
            .map(|root| root.join(file_id.as_ref()))
            .context("logging configs resource is not available")
    }

    pub async fn ensure_version<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<VersionStorage<L, VL>>
    where
        L: Clone,
        VL: Layout,
    {
        let name = name.into();
        let versions_root = self.versions_root_path()?;
        create_dir_all(&versions_root).await?;

        let version_root = versions_root.join(&name);
        create_dir_all(&version_root).await?;

        Ok(VersionStorage::new(
            version_root,
            self.clone(),
            version_layout,
        ))
    }

    pub async fn write_asset_index(
        &self,
        id: impl Display,
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

    pub fn version_exists(&self, name: impl AsRef<str>) -> Result<bool> {
        let name = name.as_ref();
        let version_root = self.version_root_path(name)?;
        Ok(version_root.join(format!("{name}.jar")).exists()
            && version_root.join(format!("{name}.json")).exists())
    }

    pub fn version<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<VersionStorage<L, VL>>
    where
        L: Clone,
        VL: Layout,
    {
        let name = name.into();
        if !self.version_exists(&name)? {
            return Err(anyhow::anyhow!("can't find a valid version named '{name}'"));
        }

        Ok(VersionStorage::new(
            self.version_root_path(&name)?,
            self.clone(),
            version_layout,
        ))
    }

    pub fn version_names(&self) -> Result<Vec<String>> {
        let versions_root = self.versions_root_path()?;
        if !versions_root.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in versions_root.read_dir()? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if self.version_exists(&name)? {
                versions.push(name);
            }
        }

        Ok(versions)
    }

    pub fn locate(&self) {}
}

impl<L: Layout> Layoutable<L> for GameStorage<L> {
    fn layout(&self) -> &L {
        &self.layout
    }

    fn root_path(&self) -> &Path {
        &self.path
    }
}
