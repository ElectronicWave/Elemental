use elemental_schema::fabric::ProfileLibrary;

use crate::{
    error::{Error, Result},
    maven::MavenNotation,
};

use super::super::manifest::{LiteLoaderLibraryResolution, LiteLoaderManifestLibrary};
use super::super::source::LiteLoaderEndpoints;
use super::library::{
    default_snapshot_repository, fixed_library_repository, profile_library_with_repository,
    rewritten_repository,
};

pub(super) async fn normalize_manifest_library(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    library: LiteLoaderManifestLibrary,
) -> Result<ProfileLibrary> {
    let notation = MavenNotation::parse(library.name.as_str(), "liteloader library")?;
    if let Some(repository) = fixed_library_repository(endpoints, notation.group())? {
        return Ok(profile_library_with_repository(
            library.name,
            &repository,
            None,
        ));
    }

    let repository = match (library.resolution, library.url.is_empty()) {
        (LiteLoaderLibraryResolution::Snapshot, true) => {
            default_snapshot_repository(endpoints, &notation, library.name.as_str())?
        }
        (_, false) => {
            rewritten_repository(endpoints, library.url.as_str(), "liteloader repository")?
        }
        (LiteLoaderLibraryResolution::Release, true) => {
            return Err(Error::LiteLoaderLibraryRepositoryMissing {
                library: library.name,
            });
        }
    };
    let artifact = if library.resolution == LiteLoaderLibraryResolution::Snapshot {
        Some(
            repository
                .resolve_snapshot_artifact(client, &notation)
                .await?,
        )
    } else {
        None
    };

    Ok(profile_library_with_repository(
        library.name,
        &repository,
        artifact,
    ))
}
