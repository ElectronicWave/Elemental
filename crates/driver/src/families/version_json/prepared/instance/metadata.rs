use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonAssetIndexStorageExt, VersionJsonInstanceLayout, VersionJsonMetadataStorageExt,
        VersionJsonRootLayout, remote::VersionJsonRemoteResolver,
    },
};

use super::super::{ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata};

impl<R: VersionJsonRemoteResolver> ResolvedVersionJsonMetadata<R> {
    pub fn new(
        remote_resolver: R,
        metadata: PistonMetaData,
        asset_index_objects: PistonMetaAssetIndexObjects,
    ) -> Self {
        Self {
            remote_resolver,
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
    ) -> Result<ResolvedVersionJsonInstance<R, L, VL>> {
        persist_version_metadata(instance, &self.metadata, &self.asset_index_objects).await?;
        Ok(ResolvedVersionJsonInstance {
            remote_resolver: self.remote_resolver,
            version: instance.clone(),
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}

pub(crate) async fn persist_version_metadata<
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
>(
    instance: &elemental_core::storage::Storage<VL, elemental_core::storage::Storage<L>>,
    metadata: &PistonMetaData,
    asset_index_objects: &PistonMetaAssetIndexObjects,
) -> Result<()> {
    instance.write_metadata(metadata).await?;
    instance
        .parent
        .write_asset_index(metadata.asset_index.id.clone(), asset_index_objects)
        .await
}
