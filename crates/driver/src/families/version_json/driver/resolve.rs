use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};

use crate::{
    drivers::vanilla::source::resolve_vanilla_metadata, error::Result,
    loader_version::LoaderVersionId,
};

use super::super::{
    ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata, VersionJsonInstanceLayout,
    VersionJsonRootLayout, family::ProfiledVersionJsonFamily,
};
use super::ProfiledVersionJsonDriver;

impl<F> ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    pub(super) async fn resolve_or_load<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<ResolvedVersionJsonInstance<F::RemoteResolver, L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        if let Ok(resolved) =
            ResolvedVersionJsonInstance::load(self.remote_resolver(), instance.clone())
        {
            let status = resolved.status().await?;
            if status.is_downloaded()
                && !self.family.local_metadata_needs_refresh(
                    &resolved.metadata,
                    &game_version,
                    &loader_version,
                )
            {
                return Ok(resolved);
            }
        }

        self.resolve_metadata(game_version, loader_version)
            .await?
            .persist(instance)
            .await
    }

    async fn resolve_metadata(
        &self,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<ResolvedVersionJsonMetadata<F::RemoteResolver>> {
        let base_metadata =
            resolve_vanilla_metadata(&self.vanilla_source, game_version.as_str()).await?;
        let profile = self
            .family
            .profile(&self.source, &game_version, &loader_version)
            .await?;
        let metadata = self.family.merge_profile(base_metadata.metadata, profile)?;

        Ok(ResolvedVersionJsonMetadata::new(
            self.remote_resolver(),
            metadata,
            base_metadata.asset_index_objects,
        ))
    }
}
