use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_schema::mojang::piston::{PistonMetaData, PistonMetaLibrariesDownloadsArtifact};

use crate::{
    error::Result,
    families::version_json::{
        VersionJsonVersionStorageExt,
        extensions::PistonMetaLibrariesExt,
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::VersionJsonInstanceResource,
        rules::VersionJsonRuleContext,
        state::{NativesState, natives_state_store},
    },
};

pub(super) struct NativesExpectation {
    destination: PathBuf,
    native_artifacts: Vec<String>,
}

pub(super) fn expected_natives<L, VL>(
    storage: &Storage<VL, Storage<L>>,
) -> Option<NativesExpectation>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let metadata = VersionJsonVersionStorageExt::metadata(storage).ok()?;
    let rule_context = VersionJsonRuleContext::current();
    let destination = storage
        .try_get_resource(VersionJsonInstanceResource::Natives)
        .ok()?;

    Some(NativesExpectation {
        destination,
        native_artifacts: collect_native_artifact_paths(&metadata, &rule_context),
    })
}

pub(super) async fn record_natives_state<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    destination: &Path,
    rule_context: &VersionJsonRuleContext,
    extracted_files: Vec<String>,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
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

pub(super) fn natives_state_matches(state: &NativesState, expected: &NativesExpectation) -> bool {
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

pub(super) fn native_source_artifacts<'a>(
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
