mod artifact;
mod coordinate;
mod repository;
mod snapshot;
mod url;

pub use artifact::maven_library_downloads;
pub use coordinate::MavenCoordinate;
pub use repository::MavenRepository;
pub use snapshot::fetch_maven_metadata;

use elemental_infra::SafeRelativePath;

use crate::error::{Error, Result};

pub(crate) fn validate_component(value: &str, label: &str, notation: &str) -> Result<()> {
    if !coordinate::is_valid_component(value) {
        return Err(Error::InvalidNotation {
            label: label.to_owned(),
            notation: notation.to_owned(),
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenArtifactDownload {
    pub path: SafeRelativePath,
    pub url: String,
}
