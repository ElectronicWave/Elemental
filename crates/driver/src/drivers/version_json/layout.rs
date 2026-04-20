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

pub trait VersionJsonRootLayout: Layout {
    fn instances_root_path(&self, root: &Path) -> PathBuf;
    fn asset_index_path(&self, root: &Path, id: &str) -> PathBuf;
    fn asset_object_path(&self, root: &Path, hash: &str) -> PathBuf;
    fn library_path(&self, root: &Path, path: &Path) -> PathBuf;
    fn logging_config_path(&self, root: &Path, file_id: &str) -> PathBuf;
}

pub trait VersionJsonInstanceLayout: Layout {
    fn metadata_path(&self, root: &Path, name: &str) -> PathBuf {
        root.join(format!("{name}.json"))
    }

    fn jar_path(&self, root: &Path, name: &str) -> PathBuf {
        root.join(format!("{name}.jar"))
    }

    fn platform_natives_path(&self, root: &Path) -> PathBuf {
        root.join("natives")
    }

    fn natives_marker_path(&self, root: &Path) -> PathBuf {
        root.join(".elemental-natives-ready")
    }
}

impl VersionJsonRootLayout for BaseLayout {
    fn instances_root_path(&self, root: &Path) -> PathBuf {
        root.join("versions")
    }

    fn asset_index_path(&self, root: &Path, id: &str) -> PathBuf {
        root.join("assets")
            .join("indexes")
            .join(format!("{id}.json"))
    }

    fn asset_object_path(&self, root: &Path, hash: &str) -> PathBuf {
        let prefix = hash
            .get(0..2)
            .expect("asset hash is too short for version-json layout");
        root.join("assets").join("objects").join(prefix).join(hash)
    }

    fn library_path(&self, root: &Path, path: &Path) -> PathBuf {
        root.join("libraries").join(path)
    }

    fn logging_config_path(&self, root: &Path, file_id: &str) -> PathBuf {
        root.join("assets").join("log_configs").join(file_id)
    }
}

impl<L: Layout> VersionJsonInstanceLayout for L {}
