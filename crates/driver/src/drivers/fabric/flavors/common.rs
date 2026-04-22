use anyhow::Result;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::families::version_json::{
    ProfileMergeBehavior, merge_profile_with_behavior, metadata_has_replaced_library_conflicts,
};
use crate::loader_version::LoaderVersionId;

pub(super) trait FlavorBehavior: ProfileMergeBehavior {
    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        metadata_identity_mismatch(metadata, game_version, loader_version)
            || metadata_has_replaced_library_conflicts(self, metadata)
    }
}

impl FlavorBehavior for crate::families::version_json::PassthroughProfileBehavior {}

pub(super) fn merge_profile(
    behavior: &dyn FlavorBehavior,
    base_metadata: PistonMetaData,
    profile: ProfileJson,
) -> Result<PistonMetaData> {
    merge_profile_with_behavior(behavior, base_metadata, profile)
}

pub(super) fn metadata_identity_mismatch(
    metadata: &PistonMetaData,
    game_version: &MinecraftVersionId,
    loader_version: &LoaderVersionId,
) -> bool {
    let expected_id = format!("fabric-loader-{loader_version}-{game_version}");
    metadata.id != expected_id || metadata.inherits_from.as_deref() != Some(game_version.as_str())
}
