use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonAssetIndexStorageExt, VersionJsonInstanceLayout, VersionJsonMetadataStorageExt,
        VersionJsonRootLayout,
    },
};

use super::super::{ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata};

impl ResolvedVersionJsonMetadata {
    pub fn new(metadata: PistonMetaData, asset_index_objects: PistonMetaAssetIndexObjects) -> Self {
        Self {
            metadata,
            asset_index_objects,
        }
    }

    pub async fn persist<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        self,
        instance: &elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    ) -> Result<ResolvedVersionJsonInstance<L, VL>> {
        instance.write_metadata(&self.metadata).await?;
        instance
            .parent
            .write_asset_index(
                self.metadata.asset_index.id.clone(),
                &self.asset_index_objects,
            )
            .await?;
        Ok(ResolvedVersionJsonInstance {
            version: instance.clone(),
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}
