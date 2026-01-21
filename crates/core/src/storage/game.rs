use std::{fmt::Display, path::PathBuf};

use crate::storage::{
    layout::{Layout, Layoutable},
    resource::Resource,
};

pub struct GameStorage<L: Layout> {
    pub path: PathBuf,
    pub layout: L,
}

impl<L: Layout> GameStorage<L> {
    pub fn new(path: PathBuf, layout: L) -> Self {
        Self { path, layout }
    }

    pub fn objectindex(&self, id: impl Display) -> Option<PathBuf> {
        self.layout
            .get_resource(&self.path, Resource::AssetsIndexes)
            .and_then(|path| Some(path.join(format!("{}.json", id))))
    }
}

impl<L: Layout> Layoutable<L> for GameStorage<L> {
    fn layout(&self) -> &L {
        &self.layout
    }

    fn root_path(&self) -> &PathBuf {
        &self.path
    }
}
