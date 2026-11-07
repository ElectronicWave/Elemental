use crate::{
    drivers::vanilla::prepared::ResolvedVanillaMetadata,
    error::{Error, Result},
};

use super::VanillaSource;

pub async fn resolve_vanilla_metadata(
    vanilla_source: &VanillaSource,
    game_version: &str,
) -> Result<ResolvedVanillaMetadata> {
    let launchmeta = vanilla_source.launch_meta().await?;
    let metadata_url = launchmeta
        .versions
        .iter()
        .find(|version| version.id == game_version)
        .ok_or_else(|| Error::VanillaVersionNotFound {
            game_version: game_version.to_owned(),
        })?
        .url
        .clone();
    let metadata = vanilla_source.piston_meta(metadata_url).await?;
    let asset_index_objects = vanilla_source
        .asset_index_objects(&metadata.asset_index.url)
        .await?;

    Ok(ResolvedVanillaMetadata::new(
        vanilla_source.endpoints().clone(),
        metadata,
        asset_index_objects,
    ))
}
