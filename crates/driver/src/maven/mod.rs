mod notation;
mod path;
mod repository;
mod snapshot;
mod url;
mod validation;

pub use notation::MavenNotation;
pub use repository::MavenRepository;
pub use snapshot::fetch_maven_metadata;

use elemental_infra::SafeRelativePath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenArtifactDownload {
    pub path: SafeRelativePath,
    pub url: String,
}
