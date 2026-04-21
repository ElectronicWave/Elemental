use std::path::{Path, PathBuf};

use tokio::fs::create_dir_all;

use anyhow::Result;

use crate::storage::layout::{Layout, Layoutable};

#[derive(Debug, Clone)]
pub struct Storage<L: Layout, P = ()> {
    pub path: PathBuf,
    pub parent: P,
    pub layout: L,
}

impl<L: Layout> Storage<L> {
    pub fn new<P: AsRef<Path>>(path: P, layout: L) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            parent: (),
            layout,
        }
    }

    pub fn new_ensure_dir<P: AsRef<Path>>(path: P, layout: L) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            path,
            parent: (),
            layout,
        })
    }
}

impl<L: Layout, P> Storage<L, P> {
    pub fn with_parent(path: PathBuf, parent: P, layout: L) -> Self {
        Self {
            path,
            parent,
            layout,
        }
    }

    pub fn name(&self) -> Option<String> {
        self.path
            .file_name()
            .and_then(|n| n.to_str().map(|s| s.to_string()))
    }

    pub async fn ensure_root(&self) -> Result<()> {
        create_dir_all(&self.path).await?;
        Ok(())
    }

    pub fn scope<NL>(&self, relative: impl AsRef<Path>, layout: NL) -> Storage<NL, Self>
    where
        NL: Layout,
        Self: Clone,
    {
        Storage::with_parent(self.path.join(relative), self.clone(), layout)
    }
}

impl<L: Layout, P> Layoutable<L> for Storage<L, P> {
    fn layout(&self) -> &L {
        &self.layout
    }

    fn root_path(&self) -> &Path {
        &self.path
    }
}
