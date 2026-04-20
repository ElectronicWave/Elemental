use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs::create_dir_all;

pub trait Layout: Send + Sync {
    type Resource: Send + 'static;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf>;
    fn name(&self) -> &'static str;
}

#[async_trait::async_trait]
pub trait Layoutable<L: Layout> {
    fn layout(&self) -> &L;
    fn root_path(&self) -> &Path;

    fn get_resource(&self, resource: L::Resource) -> Option<PathBuf> {
        self.layout().get_resource(self.root_path(), resource)
    }

    fn get_existing_resource(&self, resource: L::Resource) -> Option<PathBuf> {
        self.get_resource(resource)
            .and_then(|p| if p.exists() { Some(p) } else { None })
    }

    async fn ensure_resource(&self, resource: L::Resource) -> Result<Option<PathBuf>> {
        if let Some(path) = self.get_resource(resource) {
            if path.exists() {
                return Ok(Some(path));
            } else {
                create_dir_all(&path).await?;
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
}
