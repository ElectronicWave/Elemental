use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;

use super::BaseInstanceLayout;
use crate::families::version_json::VersionJsonInstanceResource;

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
            VersionJsonInstanceResource::Elemental(Some(path)) => {
                Some(root.join(".elemental").join(path.as_path()))
            }
            VersionJsonInstanceResource::Elemental(None) => Some(root.join(".elemental")),
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
