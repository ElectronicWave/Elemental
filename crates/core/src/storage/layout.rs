use std::path::{Path, PathBuf};

use crate::storage::resource::Resource;
use anyhow::Result;
use tokio::fs::create_dir_all;

pub trait Layout: Send + Sync {
    fn get_resource(&self, root: &Path, resource: Resource) -> Option<PathBuf>;
    fn name(&self) -> &'static str;
}

#[async_trait::async_trait]
pub trait Layoutable<L: Layout> {
    fn layout(&self) -> &L;
    fn root_path(&self) -> &Path;
    fn get_resource(&self, resource: Resource) -> Option<PathBuf> {
        self.layout().get_resource(self.root_path(), resource)
    }

    fn get_existing_resource(&self, resource: Resource) -> Option<PathBuf> {
        self.get_resource(resource)
            .and_then(|p| if p.exists() { Some(p) } else { None })
    }

    async fn ensure_resource(&self, resource: Resource) -> Result<Option<PathBuf>> {
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

#[derive(Debug, Default)]
pub struct BaseLayout;

impl Layout for BaseLayout {
    fn get_resource(&self, root: &Path, resource: Resource) -> Option<PathBuf> {
        match resource {
            Resource::AssetsIndexes => Some(root.join("assets").join("indexes")),
            Resource::AssetsObjects => Some(root.join("assets").join("objects")),
            Resource::Versions => Some(root.join("versions")),
            Resource::Logs => Some(root.join("logs")),
            Resource::Configs => Some(root.join("config")),
            Resource::ShaderPacks => Some(root.join("shaderpacks")),
            Resource::ResourcePacks => Some(root.join("resourcepacks")),
            Resource::Saves => Some(root.join("saves")),
            Resource::Libraries => Some(root.join("libraries")),
            Resource::Natives => Some(root.join("natives")),
            Resource::Mods => Some(root.join("mods")),
            Resource::Dot => Some(root.to_path_buf()),
            Resource::Subdir(subdir) => Some(root.join(subdir)),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "Base"
    }
}
