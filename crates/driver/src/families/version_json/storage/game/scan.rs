use std::path::Path;

use elemental_core::storage::Storage;

use crate::{
    error::Result,
    families::version_json::layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
};

use super::VersionJsonGameStorageExt;

pub(super) fn instance_names_in<L, VL>(
    storage: &Storage<L>,
    root: &Path,
    version_layout: &VL,
) -> Result<Vec<String>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let mut instances = Vec::new();
    for entry in root.read_dir()? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if storage.instance_exists(&name, version_layout)? {
            instances.push(name);
        }
    }
    Ok(instances)
}
