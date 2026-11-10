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
    directory::{NativesDirectory, relative_path_key},
    state::{
        expected_natives, native_library_sources, natives_state_matches, record_natives_state,
    },
};

pub(crate) async fn natives_are_extracted<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let expected = expected_natives(storage, metadata)?;
    if expected.native_artifacts.is_empty() {
        return Ok(true);
    }

    let store = natives_state_store(&storage.path).await?;
    Ok(natives_state_matches(
        &store.cloned().await.value,
        &expected,
    ))
}

pub(crate) async fn ensure_natives_extracted<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    if natives_are_extracted(storage, metadata).await? {
        return Ok(());
    }

    extract_natives_with_metadata(storage, metadata).await
}

pub(super) async fn extract_natives_with_metadata<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let destination = storage.try_get_resource(VersionJsonInstanceResource::Natives)?;
    let rule_context = VersionJsonRuleContext::current();
    std::fs::create_dir_all(&destination)?;

    let extracted_files = extract_native_libraries(storage, metadata, &destination, &rule_context)?;
    record_natives_state(
        storage,
        metadata,
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
        for source in native_library_sources(metadata, rule_context)? {
            extract_native_library(storage, &source.artifact, destination, &source.exclude)?;
        }
        NativesDirectory::new(destination).binary_paths()
    })
}

fn extract_native_library<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    artifact: &PistonMetaLibrariesDownloadsArtifact,
    destination: &Path,
    exclude: &[String],
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
        .extract_missing_blocking(destination, |relative_path| {
            NativesDirectory::is_binary(relative_path)
                && !is_excluded_relative_path(relative_path, exclude)
        })
        .map(|_| ())
        .map_err(Error::from)
}

fn is_excluded_relative_path(relative_path: &Path, exclude: &[String]) -> bool {
    let relative_path = relative_path_key(relative_path);
    exclude
        .iter()
        .any(|excluded| relative_path.starts_with(excluded))
}
