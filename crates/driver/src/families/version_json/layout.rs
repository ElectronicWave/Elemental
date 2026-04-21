use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;

use super::resource::{VersionJsonInstanceResource, VersionJsonRootResource};

#[derive(Debug, Clone, Default)]
pub struct BaseRootLayout;

#[derive(Debug, Clone, Default)]
pub struct BaseInstanceLayout;

impl Layout for BaseRootLayout {
    type Resource = VersionJsonRootResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        let assets_root = root.join("assets");
        let asset_indexes_root = assets_root.join("indexes");
        let asset_objects_root = assets_root.join("objects");
        let asset_log_configs_root = assets_root.join("log_configs");
        let versions_root = root.join("versions");
        let libraries_root = root.join("libraries");

        match resource {
            VersionJsonRootResource::Assets => Some(assets_root),
            VersionJsonRootResource::AssetIndexes(id) => match id.filter(|id| !id.is_empty()) {
                Some(id) => Some(asset_indexes_root.join(format!("{id}.json"))),
                None => Some(asset_indexes_root),
            },
            VersionJsonRootResource::AssetObjects(hash) => {
                match hash.filter(|hash| !hash.is_empty()) {
                    Some(hash) => {
                        let prefix = hash.get(0..2)?;
                        Some(asset_objects_root.join(prefix).join(hash))
                    }
                    None => Some(asset_objects_root),
                }
            }
            VersionJsonRootResource::AssetLogConfigs(id) => match id.filter(|id| !id.is_empty()) {
                Some(id) => Some(asset_log_configs_root.join(id)),
                None => Some(asset_log_configs_root),
            },
            VersionJsonRootResource::Versions(name) => match name.filter(|name| !name.is_empty()) {
                Some(name) => Some(versions_root.join(name)),
                None => Some(versions_root),
            },
            VersionJsonRootResource::Libraries(path) => {
                match path.filter(|path| !path.as_os_str().is_empty()) {
                    Some(path) => Some(libraries_root.join(path)),
                    None => Some(libraries_root),
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "BaseRoot"
    }
}

impl Layout for BaseInstanceLayout {
    type Resource = VersionJsonInstanceResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        let name = root.file_name()?.to_string_lossy().to_string();

        match resource {
            VersionJsonInstanceResource::Metadata => Some(root.join(format!("{name}.json"))),
            VersionJsonInstanceResource::Jar => Some(root.join(format!("{name}.jar"))),
            VersionJsonInstanceResource::Natives => Some(root.join("natives")),
            VersionJsonInstanceResource::Logs => Some(root.join("logs")),
            VersionJsonInstanceResource::Configs => Some(root.join("config")),
            VersionJsonInstanceResource::ShaderPacks => Some(root.join("shaderpacks")),
            VersionJsonInstanceResource::ResourcePacks => Some(root.join("resourcepacks")),
            VersionJsonInstanceResource::Saves => Some(root.join("saves")),
            VersionJsonInstanceResource::Mods => Some(root.join("mods")),
        }
    }

    fn name(&self) -> &'static str {
        "BaseInstance"
    }
}

pub trait VersionJsonRootLayout: Layout<Resource = VersionJsonRootResource> {}

pub trait VersionJsonInstanceLayout: Layout<Resource = VersionJsonInstanceResource> {}

impl<L> VersionJsonRootLayout for L where L: Layout<Resource = VersionJsonRootResource> {}

impl<L> VersionJsonInstanceLayout for L where L: Layout<Resource = VersionJsonInstanceResource> {}
