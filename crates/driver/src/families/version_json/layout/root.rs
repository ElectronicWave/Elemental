use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;

use super::BaseRootLayout;
use crate::families::version_json::VersionJsonRootResource;

impl Layout for BaseRootLayout {
    type Resource = VersionJsonRootResource;
    type ExtendedResource = VersionJsonRootResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        let assets_root = root.join("assets");
        let asset_indexes_root = assets_root.join("indexes");
        let asset_objects_root = assets_root.join("objects");
        let asset_log_configs_root = assets_root.join("log_configs");
        let versions_root = root.join("versions");
        let libraries_root = root.join("libraries");

        match resource {
            VersionJsonRootResource::Assets => Some(assets_root),
            VersionJsonRootResource::AssetIndexes(id) => match id {
                Some(id) => Some(asset_indexes_root.join(format!("{}.json", id.as_str()))),
                None => Some(asset_indexes_root),
            },
            VersionJsonRootResource::AssetObjects(hash) => match hash {
                Some(hash) => {
                    let prefix = hash.as_str().get(0..2)?;
                    Some(asset_objects_root.join(prefix).join(hash.as_path()))
                }
                None => Some(asset_objects_root),
            },
            VersionJsonRootResource::AssetLogConfigs(id) => match id {
                Some(id) => Some(asset_log_configs_root.join(id.as_path())),
                None => Some(asset_log_configs_root),
            },
            VersionJsonRootResource::Versions(name) => match name {
                Some(name) => Some(versions_root.join(name.as_path())),
                None => Some(versions_root),
            },
            VersionJsonRootResource::Libraries(path) => match path {
                Some(path) => Some(libraries_root.join(path.as_path())),
                None => Some(libraries_root),
            },
        }
    }

    fn name(&self) -> &'static str {
        "BaseRoot"
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.get_resource(root, resource)
    }
}
