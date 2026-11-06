use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::jar::JarFile;
use elemental_schema::mojang::piston::{PistonMetaData, PistonMetaLibrariesDownloadsArtifact};

use crate::{
    error::{Error, Result},
    families::version_json::{
        extensions::PistonMetaLibrariesExt,
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::{VersionJsonInstanceResource, VersionJsonRootResource},
        rules::VersionJsonRuleContext,
        state::{NativesState, natives_state_store},
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

    extract_native_libraries(storage, &metadata, &destination, &rule_context)?;
    record_natives_state(storage, &metadata, &destination, &rule_context).await
}

fn expected_natives<L, VL>(storage: &Storage<VL, Storage<L>>) -> Option<NativesExpectation>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let metadata = super::VersionJsonVersionStorageExt::metadata(storage).ok()?;
    let rule_context = VersionJsonRuleContext::current();
    let destination = storage
        .try_get_resource(VersionJsonInstanceResource::Natives)
        .ok()?;

    Some(NativesExpectation {
        destination,
        native_artifacts: collect_native_artifact_paths(&metadata, &rule_context),
    })
}

fn extract_native_libraries<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    destination: &Path,
    rule_context: &VersionJsonRuleContext,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    tokio::task::block_in_place(|| {
        for artifact in native_source_artifacts(metadata, rule_context) {
            let source = storage
                .parent
                .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                    artifact.path.as_str(),
                ))))?;
            JarFile::new(source).extract_blocking(destination)?;
        }
        NativesDirectory::new(destination).flatten_binaries()
    })
}

async fn record_natives_state<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    destination: &Path,
    rule_context: &VersionJsonRuleContext,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let extracted_files = NativesDirectory::new(destination)
        .root_binaries()?
        .into_iter()
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .collect();
    let checked_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    natives_state_store(&storage.path)
        .await?
        .set(|state| {
            state.value = NativesState {
                path: destination.to_string_lossy().to_string(),
                native_artifacts: collect_native_artifact_paths(metadata, rule_context),
                extracted_files,
                checked_at_unix_ms,
            };
        })
        .await?;
    Ok(())
}

struct NativesExpectation {
    destination: PathBuf,
    native_artifacts: Vec<String>,
}

fn natives_state_matches(state: &NativesState, expected: &NativesExpectation) -> bool {
    !state.path.is_empty()
        && state.path == expected.destination.to_string_lossy()
        && state.native_artifacts == expected.native_artifacts
        && (expected.native_artifacts.is_empty()
            || (!state.extracted_files.is_empty()
                && state
                    .extracted_files
                    .iter()
                    .all(|file| expected.destination.join(file).exists())))
}

fn collect_native_artifact_paths(
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
) -> Vec<String> {
    let mut paths = native_source_artifacts(metadata, rule_context)
        .map(|artifact| artifact.path.clone())
        .collect::<Vec<String>>();
    paths.sort();
    paths.dedup();
    paths
}

fn native_source_artifacts<'a>(
    metadata: &'a PistonMetaData,
    rule_context: &'a VersionJsonRuleContext,
) -> impl Iterator<Item = &'a PistonMetaLibrariesDownloadsArtifact> + 'a {
    metadata
        .libraries
        .iter()
        .filter(|library| library.is_allowed(rule_context))
        .flat_map(|library| library.native_source_artifacts(rule_context.platform()))
        .flatten()
}

#[derive(Debug, Clone, Copy)]
struct NativesDirectory<'a> {
    root: &'a Path,
}

impl<'a> NativesDirectory<'a> {
    fn new(root: &'a Path) -> Self {
        Self { root }
    }

    fn flatten_binaries(self) -> Result<()> {
        for path in self.binaries()? {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let target = self.root.join(file_name);
            if target != path {
                std::fs::copy(&path, &target).map_err(|source| Error::CopyNativeBinary {
                    source_path: path,
                    target_path: target,
                    source,
                })?;
            }
        }
        Ok(())
    }

    fn root_binaries(self) -> Result<Vec<PathBuf>> {
        Ok(std::fs::read_dir(self.root)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .map(|entry| entry.path())
            .filter(|path| Self::is_binary(path))
            .collect())
    }

    fn binaries(self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.collect_binaries_into(self.root, &mut files)?;
        Ok(files)
    }

    fn collect_binaries_into(self, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                self.collect_binaries_into(&path, files)?;
            } else if Self::is_binary(&path) {
                files.push(path);
            }
        }
        Ok(())
    }

    fn is_binary(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "dll" | "so" | "dylib" | "jnilib"
                )
            })
    }
}
