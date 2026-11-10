use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_schema::mojang::piston::{PistonMetaData, PistonMetaLibrariesDownloadsArtifact};

use crate::{
    error::Result,
    families::version_json::{
        extensions::PistonMetaLibrariesExt,
        layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
        resource::VersionJsonInstanceResource,
        rules::VersionJsonRuleContext,
        state::{NativesState, natives_state_store},
    },
};

pub(super) struct NativesExpectation {
    pub(super) destination: PathBuf,
    pub(super) native_artifacts: Vec<String>,
}

pub(super) struct NativeLibrarySource {
    pub(super) artifact: PistonMetaLibrariesDownloadsArtifact,
    pub(super) exclude: Vec<String>,
}

pub(super) fn expected_natives<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
) -> Result<NativesExpectation>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let rule_context = VersionJsonRuleContext::current();
    let destination = storage.try_get_resource(VersionJsonInstanceResource::Natives)?;

    Ok(NativesExpectation {
        destination,
        native_artifacts: collect_native_artifact_paths(metadata, &rule_context)?,
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
    let native_artifacts = collect_native_artifact_paths(metadata, rule_context)?;
    natives_state_store(&storage.path)
        .await?
        .set(|state| {
            state.value = NativesState {
                path: destination.to_string_lossy().to_string(),
                native_artifacts,
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
) -> Result<Vec<String>> {
    let mut paths = native_library_sources(metadata, rule_context)?
        .into_iter()
        .map(|source| source.artifact.path)
        .collect::<Vec<String>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub(super) fn native_library_sources(
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
) -> Result<Vec<NativeLibrarySource>> {
    let mut sources = Vec::new();
    for library in metadata
        .libraries
        .iter()
        .filter(|library| library.is_allowed(rule_context))
    {
        let exclude = library
            .extract
            .as_ref()
            .map(|extract| extract.exclude.clone())
            .unwrap_or_default();
        sources.extend(
            library
                .native_source_artifacts(rule_context.platform())?
                .into_iter()
                .map(|artifact| NativeLibrarySource {
                    artifact,
                    exclude: exclude.clone(),
                }),
        );
    }

    Ok(sources)
}
