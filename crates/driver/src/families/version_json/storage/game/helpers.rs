use std::path::{Path, PathBuf};

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::SafeRelativePath;

use crate::{
    error::Result,
    families::version_json::{
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::{VersionJsonInstanceResource, VersionJsonRootResource},
    },
};

pub(super) fn instance_root<L>(storage: &Storage<L>, name: impl AsRef<str>) -> Result<PathBuf>
where
    L: VersionJsonRootLayout,
{
    Ok(
        storage.try_get_resource(VersionJsonRootResource::Versions(Some(
            SafeRelativePath::parse(name.as_ref())?,
        )))?,
    )
}

pub(super) fn versions_root<L>(storage: &Storage<L>) -> Result<PathBuf>
where
    L: VersionJsonRootLayout,
{
    Ok(storage.try_get_resource(VersionJsonRootResource::Versions(None))?)
}

pub(super) fn instance_ready<VL>(root: &Path, version_layout: &VL) -> Result<bool>
where
    VL: VersionJsonInstanceLayout,
{
    Ok(version_layout
        .try_get_resource(root, VersionJsonInstanceResource::Metadata)?
        .exists()
        && version_layout
            .try_get_resource(root, VersionJsonInstanceResource::Jar)?
            .exists())
}
