use std::fs::File;

use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaAssetIndexObjects;
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::version_json::{layout::VersionJsonRootLayout, resource::VersionJsonRootResource},
};

#[async_trait(?Send)]
pub trait VersionJsonAssetIndexStorageExt {
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()>;
    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects>;
}

#[async_trait(?Send)]
impl<L> VersionJsonAssetIndexStorageExt for Storage<L>
where
    L: VersionJsonRootLayout,
{
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()> {
        let path = self.try_get_resource(VersionJsonRootResource::AssetIndexes(Some(
            SafeRelativePath::parse(id)?,
        )))?;
        let parent = path.parent().ok_or_else(|| Error::MissingParentDirectory {
            context: "asset index path has no parent directory".to_owned(),
            path: path.clone(),
        })?;
        create_dir_all(parent).await?;
        tokio::fs::write(path, serde_json::to_vec(objects)?).await?;
        Ok(())
    }

    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects> {
        let path = self.try_get_resource(VersionJsonRootResource::AssetIndexes(Some(
            SafeRelativePath::parse(id.as_ref())?,
        )))?;
        Ok(serde_json::from_reader(File::open(path)?)?)
    }
}
