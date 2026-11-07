use std::{fs::File, path::PathBuf};

use elemental_core::storage::{
    Storage,
    layout::{Layout, Layoutable},
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{VersionJsonInstanceLayout, VersionJsonInstanceResource},
};

#[derive(Debug, Clone)]
pub struct InstanceProbe<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub metadata: Option<PistonMetaData>,
}

impl<L: Layout, VL: VersionJsonInstanceLayout> InstanceProbe<L, VL> {
    pub fn collect(storage: Storage<VL, Storage<L>>) -> Result<Self> {
        let metadata_path = Self::detect_metadata_path(&storage)?;
        let metadata: Option<PistonMetaData> = match metadata_path.as_ref() {
            Some(path) => Some(serde_json::from_reader(File::open(path)?)?),
            None => None,
        };

        Ok(Self { storage, metadata })
    }

    fn detect_metadata_path(storage: &Storage<VL, Storage<L>>) -> Result<Option<PathBuf>> {
        let preferred = storage.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        if preferred.exists() {
            return Ok(Some(preferred));
        }

        let mut json_files = Vec::new();
        if storage.path.exists() {
            for entry in storage.path.read_dir()? {
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && entry
                        .path()
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                {
                    json_files.push(entry.path());
                }
            }
        }

        Ok(if json_files.len() == 1 {
            json_files.into_iter().next()
        } else {
            None
        })
    }
}
