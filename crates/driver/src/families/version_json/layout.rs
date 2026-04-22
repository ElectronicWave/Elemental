use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;

use super::resource::{VersionJsonInstanceResource, VersionJsonRootResource};

#[derive(Debug, Clone, Default)]
pub struct BaseRootLayout;

#[derive(Debug, Clone, Default)]
pub struct BaseInstanceLayout;

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

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.get_resource(root, resource)
    }
}

impl Layout for BaseInstanceLayout {
    type Resource = VersionJsonInstanceResource;
    type ExtendedResource = VersionJsonInstanceResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        match resource {
            VersionJsonInstanceResource::Metadata => {
                let name = root.file_name()?.to_string_lossy().to_string();
                Some(root.join(format!("{name}.json")))
            }
            VersionJsonInstanceResource::Jar => {
                let name = root.file_name()?.to_string_lossy().to_string();
                Some(root.join(format!("{name}.jar")))
            }
            VersionJsonInstanceResource::Natives => Some(root.join("natives")),
            VersionJsonInstanceResource::Logs => Some(root.join("logs")),
            VersionJsonInstanceResource::Configs => Some(root.join("config")),
            VersionJsonInstanceResource::ShaderPacks => Some(root.join("shaderpacks")),
            VersionJsonInstanceResource::ResourcePacks => Some(root.join("resourcepacks")),
            VersionJsonInstanceResource::Saves => Some(root.join("saves")),
            VersionJsonInstanceResource::Mods => Some(root.join("mods")),
            VersionJsonInstanceResource::Elemental(path) => match path {
                Some(path) => Some(root.join(".elemental").join(path)),
                None => Some(root.join(".elemental")),
            },
        }
    }

    fn name(&self) -> &'static str {
        "BaseInstance"
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.get_resource(root, resource)
    }
}

pub trait VersionJsonRootLayout: Layout<ExtendedResource = VersionJsonRootResource> {}

pub trait VersionJsonInstanceLayout:
    Layout<ExtendedResource = VersionJsonInstanceResource>
{
}

impl<L> VersionJsonRootLayout for L where L: Layout<ExtendedResource = VersionJsonRootResource> {}

impl<L> VersionJsonInstanceLayout for L where
    L: Layout<ExtendedResource = VersionJsonInstanceResource>
{
}
