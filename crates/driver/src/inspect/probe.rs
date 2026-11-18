use std::path::{Path, PathBuf};

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
    pub async fn collect<L, VL>(storage: &Storage<VL, Storage<L>>) -> Result<Self>
    where
        L: Layout,
        VL: VersionJsonInstanceLayout,
    {
        let preferred = storage.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        let metadata_path = match tokio::fs::metadata(&preferred).await {
            Ok(metadata) if metadata.is_file() => Some(preferred),
            Ok(_) => Self::single_json_metadata(&storage.path).await?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::single_json_metadata(&storage.path).await?
            }
            Err(error) => return Err(error.into()),
        };

        let metadata = match metadata_path {
            Some(path) => {
                let bytes = tokio::fs::read(path).await?;
                Some(serde_json::from_slice(&bytes)?)
            }
            None => None,
        };

        Ok(Self { metadata })
    }

    async fn single_json_metadata(directory: &Path) -> Result<Option<PathBuf>> {
        let mut entries = match tokio::fs::read_dir(directory).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        let mut candidates = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let is_json = path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
            if !entry.file_type().await?.is_file() || !is_json {
                continue;
            }
            candidates.push(path);
        }

        Ok(candidates.pop().filter(|_| candidates.is_empty()))
    }
}
