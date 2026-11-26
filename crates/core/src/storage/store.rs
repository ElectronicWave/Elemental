use std::path::{Path, PathBuf};

use tokio::fs::create_dir_all;

use crate::storage::{
    error::{Error, Result},
    layout::{Layout, Layoutable},
};

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

    pub fn display_name(&self) -> String {
        self.name().unwrap_or_else(|| {
            self.path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| self.path.display().to_string())
        })
    }

    pub async fn ensure_root(&self) -> Result<()> {
        create_dir_all(&self.path)
            .await
            .map_err(|source| Error::Io {
                operation: "create storage directory",
                path: self.path.clone(),
                source,
            })?;
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

impl<L: Layout, PL: Layout, PP> Storage<L, Storage<PL, PP>> {
    pub fn parent_root_path(&self) -> &Path {
        &self.parent.path
    }

    pub fn get_parent_resource(&self, resource: PL::Resource) -> Option<PathBuf> {
        self.parent.get_resource(resource)
    }

    pub fn try_get_parent_resource(&self, resource: PL::Resource) -> Result<PathBuf>
    where
        PL::Resource: std::fmt::Debug,
    {
        self.parent.try_get_resource(resource)
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
