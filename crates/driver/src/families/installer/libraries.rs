mod merge;
mod retrofutura;

use elemental_schema::mojang::piston::{PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact};

use crate::error::Result;

pub use merge::merge_libraries_prefer_embedded;
pub use retrofutura::{is_retro_futura_runtime_library, merge_retro_futura_libraries};

pub(crate) fn installer_library_artifacts(
    library: &PistonMetaLibraries,
) -> impl Iterator<Item = &PistonMetaLibrariesDownloadsArtifact> + '_ {
    library.downloads.artifact.iter().chain(
        library
            .downloads
            .classifiers
            .iter()
            .flat_map(|classifiers| classifiers.values()),
    )
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
        if let Some(artifact) = &mut library.downloads.artifact
            && artifact.url.trim().is_empty()
        {
            artifact.url = artifact_url(artifact.path.as_str())?;
        }

        if let Some(classifiers) = &mut library.downloads.classifiers {
            for artifact in classifiers.values_mut() {
                if artifact.url.trim().is_empty() {
                    artifact.url = artifact_url(artifact.path.as_str())?;
                }
            }
        }

        normalized.push(library);
    }

    Ok(normalized)
}
