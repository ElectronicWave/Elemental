use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        InstanceStorage, VersionJsonAssetIndexStorageExt, VersionJsonMetadataStorageExt,
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

    pub async fn persist(self, instance: &InstanceStorage) -> Result<ResolvedVersionJsonInstance> {
        instance.write_metadata(&self.metadata).await?;
        instance
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
