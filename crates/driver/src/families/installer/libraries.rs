mod merge;
mod retrofutura;

use elemental_schema::mojang::piston::{
    PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact, PistonMetaLibrariesSource,
};

use crate::{error::Result, families::version_json::PistonMetaLibrariesExt};

pub use merge::merge_libraries_prefer_embedded;
pub use retrofutura::{is_retro_futura_runtime_library, merge_retro_futura_libraries};

pub(crate) fn installer_library_artifacts(
    library: &PistonMetaLibraries,
) -> Result<Vec<PistonMetaLibrariesDownloadsArtifact>> {
    let downloads = library.downloads()?;
    Ok(downloads
        .artifact
        .iter()
        .chain(
            downloads
                .classifiers
                .iter()
                .flat_map(|classifiers| classifiers.values()),
        )
        .cloned()
        .collect())
}

pub fn normalize_library_urls<F>(
    libraries: Vec<PistonMetaLibraries>,
    artifact_url: F,
) -> Result<Vec<PistonMetaLibraries>>
where
    F: Fn(&str) -> Result<String>,
{
    let mut normalized = Vec::with_capacity(libraries.len());

    for mut library in libraries {
        if let PistonMetaLibrariesSource::Downloads { downloads, .. } = &mut library.source {
            for artifact in downloads.artifact.iter_mut().chain(
                downloads
                    .classifiers
                    .iter_mut()
                    .flat_map(|classifiers| classifiers.values_mut()),
            ) {
                if artifact.url.trim().is_empty() {
                    artifact.url = artifact_url(artifact.path.as_str())?;
                }
            }
        }
        normalized.push(library);
    }

    Ok(normalized)
}
