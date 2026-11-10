use std::fs::File;

use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{error::Result, families::version_json::resource::VersionJsonInstanceResource};

#[async_trait(?Send)]
pub trait VersionJsonMetadataStorageExt {
    fn metadata(&self) -> Result<PistonMetaData>;
    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()>;
}

#[async_trait(?Send)]
impl<L, VL> VersionJsonMetadataStorageExt for Storage<VL, Storage<L>>
where
    L: elemental_core::storage::layout::Layout,
    VL: elemental_core::storage::layout::Layout<Resource = VersionJsonInstanceResource>,
{
    fn metadata(&self) -> Result<PistonMetaData> {
        let path = self.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        Ok(serde_json::from_reader(File::open(path)?)?)
    }

    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()> {
        self.ensure_root().await?;
        let path = self.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        tokio::fs::write(path, serde_json::to_vec(metadata)?).await?;
        Ok(())
    }
}
