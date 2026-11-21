use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};

use crate::{
    drivers::vanilla::source::resolve_vanilla_metadata, error::Result,
    loader_version::LoaderVersionId,
};

use super::super::{
    ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata, VersionJsonDownloadStatus,
    VersionJsonInstanceLayout, VersionJsonRootLayout, family::ProfiledVersionJsonFamily,
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
    ) -> Result<(
        ResolvedVersionJsonInstance<L, VL>,
        Option<VersionJsonDownloadStatus>,
    )>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        if let Some((resolved, status)) = try_load_downloaded_instance(instance)?
            && !self.family.local_metadata_needs_refresh(
                &resolved.metadata,
                &game_version,
                &loader_version,
            )
        {
            return Ok((resolved, Some(status)));
        }

        let base_metadata =
            resolve_vanilla_metadata(&self.vanilla_source, game_version.as_str()).await?;
        let profile = self
            .family
            .profile(&self.source, &game_version, &loader_version)
            .await?;
        let metadata = self.family.merge_profile(base_metadata.metadata, profile)?;
        let resolved =
            ResolvedVersionJsonMetadata::new(metadata, base_metadata.asset_index_objects)
                .persist(instance)
                .await?;
        Ok((resolved, None))
    }
}

fn try_load_downloaded_instance<L, VL>(
    instance: &Storage<VL, Storage<L>>,
) -> Result<
    Option<(
        ResolvedVersionJsonInstance<L, VL>,
        VersionJsonDownloadStatus,
    )>,
>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    match ResolvedVersionJsonInstance::load(instance.clone()) {
        Ok(resolved) => {
            let status = resolved.download_status()?;
            if status.is_downloaded() {
                return Ok(Some((resolved, status)));
            }
            Ok(None)
        }
        Err(error) if !error.is_not_found() => Err(error),
        Err(_) => Ok(None),
    }
}
