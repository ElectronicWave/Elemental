use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;

use super::resource::Resource;

#[derive(Debug, Clone, Default)]
pub struct BaseLayout;

impl Layout for BaseLayout {
    type Resource = Resource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        match resource {
            Resource::AssetsIndexes => Some(root.join("assets").join("indexes")),
            Resource::AssetsObjects => Some(root.join("assets").join("objects")),
            Resource::AssetsLogConfigs => Some(root.join("assets").join("log_configs")),
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
            Resource::Custom(path) => Some(root.join(path)),
        }
    }

    fn name(&self) -> &'static str {
        "Base"
    }
}
