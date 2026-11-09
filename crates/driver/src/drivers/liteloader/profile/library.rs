use elemental_schema::fabric::{ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads};

use crate::{
    error::{Error, Result},
    maven::{MavenArtifactDownload, MavenCoordinate, MavenRepository},
};

use super::super::source::LiteLoaderEndpoints;

pub(super) fn profile_library_with_repository(
    name: String,
    repository: &MavenRepository,
    artifact: Option<MavenArtifactDownload>,
) -> ProfileLibrary {
    let downloads = artifact.map(|artifact| ProfileLibraryDownloads {
        artifact: Some(ProfileLibraryArtifact {
            path: artifact.path.as_str().to_owned(),
            url: artifact.url,
            size: None,
            sha1: None,
        }),
        classifiers: None,
    });

    ProfileLibrary {
        name,
        url: repository.base_url().to_owned(),
        downloads,
        natives: None,
        extract: None,
    }
}

pub(super) fn fixed_library_repository(
    endpoints: &LiteLoaderEndpoints,
    group: &str,
) -> Result<Option<MavenRepository>> {
    if group == "net.minecraft" {
        return MavenRepository::parse(
            endpoints.mojang_libraries_url()?.as_str(),
            "Mojang libraries repository",
        )
        .map(Some);
    }

    if group == "org.ow2.asm" {
        return MavenRepository::parse(
            endpoints.maven_central_url()?.as_str(),
            "Maven Central repository",
        )
        .map(Some);
    }

    Ok(None)
}

pub(super) fn rewritten_repository(
    endpoints: &LiteLoaderEndpoints,
    raw_url: &str,
    label: &str,
) -> Result<MavenRepository> {
    MavenRepository::parse(endpoints.rewrite_upstream(raw_url)?.as_str(), label)
}

pub(super) fn default_snapshot_repository(
    endpoints: &LiteLoaderEndpoints,
    coordinate: &MavenCoordinate,
    library_name: &str,
) -> Result<MavenRepository> {
    if coordinate.group() == "org.spongepowered" {
        return MavenRepository::parse(
            endpoints.sponge_maven_url()?.as_str(),
            "Sponge Maven repository",
        );
    }

    Err(Error::LiteLoaderSnapshotRepositoryMissing {
        library: library_name.to_owned(),
    })
}
