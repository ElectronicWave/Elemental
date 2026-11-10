use std::path::{Path, PathBuf};

use elemental_core::storage::layout::Layout;

use super::BaseInstanceLayout;
use crate::families::version_json::{VersionJsonInstanceResource, platform::VersionJsonPlatform};

/// Fallback natives directory names checked after the platform-specific directory.
///
/// `natives` keeps Prism and older Elemental layouts working. PCL/XMCL use
/// `<instance>-natives`, which is derived from the instance name.
const NATIVES_DIR_CANDIDATES: &[&str] = &["natives"];

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
            VersionJsonInstanceResource::Natives => {
                let name = root.file_name()?.to_string_lossy().to_string();
                let platform = VersionJsonPlatform::current().natives_dir_name();
                let instance_scoped = format!("{name}-natives");
                let existing = std::iter::once(platform.as_str())
                    .chain(NATIVES_DIR_CANDIDATES.iter().copied())
                    .chain(std::iter::once(instance_scoped.as_str()))
                    .map(|candidate| root.join(candidate))
                    .find(|candidate| candidate.is_dir());
                if let Some(existing) = existing {
                    return Some(existing);
                }
                Some(root.join(platform))
            }
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
