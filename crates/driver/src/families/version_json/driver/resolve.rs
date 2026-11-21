use elemental_core::{minecraft::MinecraftVersionId, storage::Storage};
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    drivers::vanilla::source::resolve_vanilla_metadata, error::Result,
    loader_version::LoaderVersionId,
};

use super::super::{
    VersionJsonDownloadStatus, VersionJsonInstanceLayout, VersionJsonRootLayout,
    family::ProfiledVersionJsonFamily,
    prepared::{
        instance::{load_version_metadata, persist_version_metadata},
        readiness::check_download_status,
    },
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
        PistonMetaData,
        PistonMetaAssetIndexObjects,
        Option<VersionJsonDownloadStatus>,
    )>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        if let Some((metadata, asset_index_objects, status)) =
            try_load_downloaded_instance(instance)?
            && !self
                .family
                .local_metadata_needs_refresh(&metadata, &game_version, &loader_version)
        {
            return Ok((metadata, asset_index_objects, Some(status)));
        }

        let base_metadata =
            resolve_vanilla_metadata(&self.vanilla_source, game_version.as_str()).await?;
        let profile = self
            .family
            .profile(&self.source, &game_version, &loader_version)
            .await?;
        let metadata = self.family.merge_profile(base_metadata.metadata, profile)?;
        persist_version_metadata(instance, &metadata, &base_metadata.asset_index_objects).await?;
        Ok((metadata, base_metadata.asset_index_objects, None))
    }
}

fn try_load_downloaded_instance<L, VL>(
    instance: &Storage<VL, Storage<L>>,
) -> Result<
    Option<(
        PistonMetaData,
        PistonMetaAssetIndexObjects,
        VersionJsonDownloadStatus,
    )>,
>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    match load_version_metadata(instance) {
        Ok((metadata, asset_index_objects)) => {
            let status = check_download_status(instance, &metadata, &asset_index_objects)?;
            if status.is_downloaded() {
                return Ok(Some((metadata, asset_index_objects, status)));
            }
            Ok(None)
        }
        Err(error) if !error.is_not_found() => Err(error),
        Err(_) => Ok(None),
    }
}
