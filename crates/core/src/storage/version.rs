use std::path::PathBuf;

use crate::storage::{
    game::GameStorage,
    layout::{Layout, Layoutable},
};

pub struct VersionStorage<L: Layout, VL: Layout> {
    pub path: PathBuf,
    pub inherits: GameStorage<L>,
    pub layout: VL,
}

impl<L: Layout, VL: Layout> VersionStorage<L, VL> {
    pub fn new(path: PathBuf, inherits: GameStorage<L>, layout: VL) -> Self {
        Self {
            path,
            inherits,
            layout,
        }
    }
}

impl<L: Layout, VL: Layout> Layoutable<VL> for VersionStorage<L, VL> {
    fn layout(&self) -> &VL {
        &self.layout
    }

    fn root_path(&self) -> &PathBuf {
        &self.path
    }
}
