use std::{
    fs::File,
    path::{Path, PathBuf},
};

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
pub struct InstanceProbe {
    pub metadata: Option<PistonMetaData>,
}

impl InstanceProbe {
    pub fn collect<L, VL>(storage: &Storage<VL, Storage<L>>) -> Result<Self>
    where
        L: Layout,
        VL: VersionJsonInstanceLayout,
    {
        let preferred = storage.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        let metadata_path = if preferred.exists() {
            Some(preferred)
        } else {
            Self::single_json_metadata(&storage.path)?
        };
        let metadata = metadata_path
            .map(|path| -> Result<PistonMetaData> {
                Ok(serde_json::from_reader(File::open(path)?)?)
            })
            .transpose()?;

        Ok(Self { metadata })
    }

    fn single_json_metadata(directory: &Path) -> Result<Option<PathBuf>> {
        if !directory.exists() {
            return Ok(None);
        }

        let mut metadata_path = None;
        for entry in directory.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            let is_json = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
            if !entry.file_type()?.is_file() || !is_json {
                continue;
            }
            if metadata_path.is_some() {
                return Ok(None);
            }
            metadata_path = Some(path);
        }

        Ok(metadata_path)
    }
}
