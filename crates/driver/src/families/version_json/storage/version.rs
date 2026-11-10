use std::fs::File;

use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_schema::mojang::piston::PistonMetaData;

use super::natives;
use crate::{
    error::Result,
    families::version_json::{
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::VersionJsonInstanceResource,
    },
};

#[async_trait(?Send)]
pub trait VersionJsonVersionStorageExt {
    type GameLayout: VersionJsonRootLayout;
    type VersionLayout: VersionJsonInstanceLayout;

    fn metadata(&self) -> Result<PistonMetaData>;
    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()>;
    async fn natives_are_extracted_with_metadata(&self, metadata: &PistonMetaData) -> Result<bool>;
    async fn ensure_natives_extracted_with_metadata(&self, metadata: &PistonMetaData)
    -> Result<()>;
    async fn extract_natives_with_metadata(&self, metadata: &PistonMetaData) -> Result<()>;
}

#[async_trait(?Send)]
impl<L, VL> VersionJsonVersionStorageExt for Storage<VL, Storage<L>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    type GameLayout = L;
    type VersionLayout = VL;

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

    async fn natives_are_extracted_with_metadata(&self, metadata: &PistonMetaData) -> Result<bool> {
        natives::natives_are_extracted_with_metadata(self, metadata).await
    }

    async fn ensure_natives_extracted_with_metadata(
        &self,
        metadata: &PistonMetaData,
    ) -> Result<()> {
        natives::ensure_natives_extracted_with_metadata(self, metadata).await
    }

    async fn extract_natives_with_metadata(&self, metadata: &PistonMetaData) -> Result<()> {
        natives::extract_natives_with_metadata(self, metadata).await
    }
}
