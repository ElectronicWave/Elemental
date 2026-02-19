use std::{fs::File, path::{Path, PathBuf}};

use anyhow::{Context, Result};

use crate::storage::{
    game::GameStorage,
    layout::{Layout, Layoutable},
};
use crate::models::mojang::PistonMetaData;

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
}

impl<L: Layout, VL: Layout> Layoutable<VL> for VersionStorage<L, VL> {
    fn layout(&self) -> &VL {
        &self.layout
    }

    fn root_path(&self) -> &Path {
        &self.path
    }
}
