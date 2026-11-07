mod directory;
mod state;

use std::path::Path;

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::{SafeRelativePath, jar::JarFile};
use elemental_schema::mojang::piston::{PistonMetaData, PistonMetaLibrariesDownloadsArtifact};

use crate::{
    error::{Error, Result},
    families::version_json::{
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::{VersionJsonInstanceResource, VersionJsonRootResource},
        rules::VersionJsonRuleContext,
        state::natives_state_store,
    },
};

use self::{
    directory::NativesDirectory,
    state::{
        expected_natives, native_source_artifacts, natives_state_matches, record_natives_state,
    },
};

pub(super) async fn natives_are_extracted<L, VL>(storage: &Storage<VL, Storage<L>>) -> bool
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let Some(expected) = expected_natives(storage) else {
        return false;
    };
    let Ok(store) = natives_state_store(&storage.path).await else {
        return false;
    };
    natives_state_matches(&store.cloned().await.value, &expected)
}

pub(super) async fn extract_natives<L, VL>(storage: &Storage<VL, Storage<L>>) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let metadata = super::VersionJsonVersionStorageExt::metadata(storage)?;
    let destination = storage.try_get_resource(VersionJsonInstanceResource::Natives)?;
    let rule_context = VersionJsonRuleContext::current();
    std::fs::create_dir_all(&destination)?;

    let extracted_files =
        extract_native_libraries(storage, &metadata, &destination, &rule_context)?;
    record_natives_state(
        storage,
        &metadata,
        &destination,
        &rule_context,
        extracted_files,
    )
    .await
}

fn extract_native_libraries<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    destination: &Path,
    rule_context: &VersionJsonRuleContext,
) -> Result<Vec<String>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    tokio::task::block_in_place(|| {
        native_source_artifacts(metadata, rule_context)
            .try_for_each(|artifact| extract_native_library(storage, artifact, destination))?;
        NativesDirectory::new(destination).flatten_binaries()
    })
}

fn extract_native_library<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    artifact: &PistonMetaLibrariesDownloadsArtifact,
    destination: &Path,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let path = SafeRelativePath::parse(artifact.path.as_str())?;
    let source = storage
        .parent
        .try_get_resource(VersionJsonRootResource::Libraries(Some(path)))?;
    JarFile::new(source)
        .extract_blocking(destination)
        .map_err(Error::from)
}
