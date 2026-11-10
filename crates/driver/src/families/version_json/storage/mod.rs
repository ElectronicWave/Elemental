mod asset_index;
mod game;
mod natives;
mod version;

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;

use crate::{
    error::Result,
    families::version_json::{VersionJsonRootLayout, VersionJsonRootResource},
};

pub use asset_index::VersionJsonAssetIndexStorageExt;
pub use game::{VersionJsonGameStorageExt, inspect_instances};
pub(crate) use natives::{ensure_natives_extracted, natives_are_extracted};
pub use version::VersionJsonMetadataStorageExt;

pub(crate) fn library_artifacts_present<L, I>(
    game_storage: &Storage<L>,
    artifact_paths: I,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    I: IntoIterator<Item = SafeRelativePath>,
{
    for path in artifact_paths {
        if !game_storage
            .try_get_resource(VersionJsonRootResource::Libraries(Some(path)))?
            .exists()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
