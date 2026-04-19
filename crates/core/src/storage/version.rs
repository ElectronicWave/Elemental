use std::{
    fs::File,
    path::{Path, PathBuf},
};

use tokio::fs::create_dir_all;

use anyhow::{Context, Result};

use crate::jar::JarFile;
use crate::storage::{
    game::GameStorage,
    layout::{Layout, Layoutable},
};
use crate::{consts::PLATFORM_NATIVES_DIR_NAME, models::mojang::PistonMetaData};

#[derive(Debug, Clone)]
pub struct VersionStorage<L: Layout, VL: Layout> {
    pub path: PathBuf,
    pub global: GameStorage<L>,
    pub layout: VL,
}

impl<L: Layout, VL: Layout> VersionStorage<L, VL> {
    pub fn new(path: PathBuf, global: GameStorage<L>, layout: VL) -> Self {
        Self {
            path,
            global,
            layout,
        }
    }

    pub fn name(&self) -> Option<String> {
        self.path
            .file_name()
            .and_then(|n| n.to_str().map(|s| s.to_string()))
    }

    pub fn metadata_path(&self) -> Result<PathBuf> {
        Ok(self.path.join(format!(
            "{}.json",
            self.name().context("get version name failed")?
        )))
    }

    pub fn metadata(&self) -> Result<PistonMetaData> {
        let path = self.metadata_path()?;
        Ok(serde_json::from_reader(File::open(path)?)?)
    }

    pub fn jar_path(&self) -> Result<PathBuf> {
        Ok(self.path.join(format!(
            "{}.jar",
            self.name().context("get version name failed")?
        )))
    }

    pub async fn ensure_root(&self) -> Result<()> {
        create_dir_all(&self.path).await?;
        Ok(())
    }

    pub async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()> {
        self.ensure_root().await?;
        tokio::fs::write(self.metadata_path()?, serde_json::to_vec(metadata)?).await?;
        Ok(())
    }

    pub fn platform_natives_path(&self) -> PathBuf {
        self.path.join(PLATFORM_NATIVES_DIR_NAME)
    }

    pub async fn ensure_platform_natives_path(&self) -> Result<PathBuf> {
        let path = self.platform_natives_path();
        create_dir_all(&path).await?;
        Ok(path)
    }

    pub fn extract_natives(&self) -> Result<()> {
        let metadata = self.metadata()?;
        let destination = self.platform_natives_path();
        std::fs::create_dir_all(&destination)?;

        for library in metadata.libraries {
            if let Some(rules) = &library.rules {
                if !rules.iter().all(|rule| rule.is_allow()) {
                    continue;
                }
            }

            if let Some(artifact) = library.try_get_classifiers_native_artifact() {
                let source = self.global.library_path(artifact.path.as_str())?;
                JarFile::new(source).extract_blocking(&destination)?;
            }

            if let Some(artifact) = library.try_get_native_artifact() {
                let source = self.global.library_path(artifact.path.as_str())?;
                JarFile::new(source).extract_blocking(&destination)?;
            }
        }

        Ok(())
    }
}

impl<L: Layout, VL: Layout> Layoutable<VL> for VersionStorage<L, VL> {
    fn layout(&self) -> &VL {
        &self.layout
    }

    fn root_path(&self) -> &Path {
        &self.path
    }
}
