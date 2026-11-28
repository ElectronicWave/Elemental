use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaAssetIndexObjects;
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::version_json::{
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::VersionJsonRootResource,
    },
};

#[async_trait]
pub trait VersionJsonAssetIndexStorageExt {
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()>;
    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects>;
}

#[async_trait]
impl<L> VersionJsonAssetIndexStorageExt for Storage<L>
where
    L: VersionJsonRootLayout,
{
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()> {
        let path = self.try_get_resource(asset_index_resource(&id)?)?;
        write_asset_index_file(path, objects).await
    }

    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects> {
        let path = self.try_get_resource(asset_index_resource(id.as_ref())?)?;
        read_asset_index_file(path)
    }
}

#[async_trait]
impl<L, PL, PP> VersionJsonAssetIndexStorageExt for Storage<L, Storage<PL, PP>>
where
    L: VersionJsonInstanceLayout,
    PL: VersionJsonRootLayout,
    PP: Send + Sync,
{
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()> {
        let path = self.try_get_parent_resource(asset_index_resource(&id)?)?;
        write_asset_index_file(path, objects).await
    }

    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects> {
        let path = self.try_get_parent_resource(asset_index_resource(id.as_ref())?)?;
        read_asset_index_file(path)
    }
}

fn asset_index_resource(id: &str) -> Result<VersionJsonRootResource> {
    Ok(VersionJsonRootResource::AssetIndexes(Some(
        SafeRelativePath::parse(id)?,
    )))
}

fn read_asset_index_file(path: std::path::PathBuf) -> Result<PistonMetaAssetIndexObjects> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

async fn write_asset_index_file(
    path: std::path::PathBuf,
    objects: &PistonMetaAssetIndexObjects,
) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::MissingParentDirectory {
        context: "asset index path has no parent directory".to_owned(),
        path: path.clone(),
    })?;
    create_dir_all(parent).await?;
    tokio::fs::write(path, serde_json::to_vec(objects)?).await?;
    Ok(())
}
