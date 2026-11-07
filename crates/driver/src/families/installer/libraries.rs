mod merge;
mod retrofutura;
mod urls;

use elemental_schema::mojang::piston::{PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact};

pub use merge::merge_libraries_prefer_embedded;
pub use retrofutura::{is_retro_futura_runtime_library, merge_retro_futura_libraries};
pub use urls::normalize_library_urls;

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
