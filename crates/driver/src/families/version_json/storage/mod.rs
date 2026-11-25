mod asset_index;
mod game;
mod natives;
mod state;
mod version;

use elemental_infra::SafeRelativePath;

use crate::{
    error::Result,
    families::version_json::{InstanceStorage, VersionJsonRootResource},
};

pub use asset_index::VersionJsonAssetIndexStorageExt;
pub use game::VersionJsonGameStorageExt;
pub(crate) use natives::{ensure_natives_extracted, natives_are_extracted};
pub use version::VersionJsonMetadataStorageExt;

pub(crate) fn library_artifacts_present<I>(
    instance_storage: &InstanceStorage,
    artifact_paths: I,
) -> Result<bool>
where
    I: IntoIterator<Item = SafeRelativePath>,
{
    for path in artifact_paths {
        if !instance_storage
            .try_get_parent_resource(VersionJsonRootResource::Libraries(Some(path)))?
            .exists()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
