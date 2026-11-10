use elemental_schema::fabric::{
    ProfileJson, ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads,
};

use crate::{error::Result, inspect::ProfileIdPattern, maven::MavenCoordinate};

use super::endpoints::RiftEndpoints;
use super::release::RiftRelease;
use super::with_trailing_slash;

const RIFT_PROFILE_ID: ProfileIdPattern = ProfileIdPattern::new("-rift-");

pub(super) fn normalize_profile_json(
    mut profile: ProfileJson,
    release: &RiftRelease,
    endpoints: &RiftEndpoints,
) -> Result<ProfileJson> {
    profile.libraries = profile
        .libraries
        .into_iter()
        .map(|library| normalize_profile_library(library, endpoints))
        .collect::<Result<Vec<ProfileLibrary>>>()?;
    let asset_url =
        endpoints.release_asset_url(release.tag_name.as_str(), release.asset_name.as_str())?;
    let normalized_loader_library = build_loader_library(release, asset_url)?;
    // Rift 1.13.2 embeds a transient JitPack coordinate, so pin the selected release jar directly.
    if let Some(index) = profile
        .libraries
        .iter()
        .position(|library| is_rift_loader_library(library.name.as_str()))
    {
        profile.libraries[index] = normalized_loader_library;
    } else {
        profile.libraries.insert(0, normalized_loader_library);
    }
    profile.id = RIFT_PROFILE_ID.build(
        profile.inherits_from.as_str(),
        release.loader_version.as_str(),
    );

    Ok(profile)
}
fn normalize_profile_library(
    library: ProfileLibrary,
    endpoints: &RiftEndpoints,
) -> Result<ProfileLibrary> {
    if library.name == "org.dimdev:mixin:0.7.11-SNAPSHOT" {
        // The historical DimDev Maven endpoint is no longer a reliable source for this snapshot.
        return build_rift_mixin_library(endpoints);
    }

    if library.url == "http://repo1.maven.org/maven2/" {
        let maven_central_url = endpoints.maven_central_url()?;
        return Ok(ProfileLibrary {
            url: with_trailing_slash(maven_central_url.as_str()),
            ..library
        });
    }

    if library.url.is_empty() && library.name.starts_with("net.minecraft:") {
        return Ok(ProfileLibrary {
            url: "https://libraries.minecraft.net/".to_owned(),
            ..library
        });
    }

    Ok(library)
}

fn build_loader_library(release: &RiftRelease, asset_url: String) -> Result<ProfileLibrary> {
    let coordinate = MavenCoordinate::parse_labeled(
        &format!("org.dimdev:rift:{}", release.loader_version),
        "rift loader library",
    )?;
    let artifact_path = coordinate.artifact_path();

    Ok(ProfileLibrary {
        name: coordinate.to_string(),
        url: asset_url.clone(),
        downloads: Some(ProfileLibraryDownloads {
            artifact: Some(ProfileLibraryArtifact {
                path: artifact_path,
                url: asset_url,
                size: release.asset_size,
                sha1: None,
            }),
            classifiers: None,
        }),
        natives: None,
        extract: None,
    })
}

fn build_rift_mixin_library(endpoints: &RiftEndpoints) -> Result<ProfileLibrary> {
    let coordinate = MavenCoordinate::parse_labeled(
        "org.spongepowered:mixin:0.7.11-SNAPSHOT",
        "rift mixin library",
    )?;
    let artifact_path = coordinate.artifact_path_for_version("0.7.11-20180703.121122-1");
    let sponge_maven_url = endpoints.sponge_maven_url()?;
    let artifact_url = format!("{}/{artifact_path}", sponge_maven_url.trim_end_matches('/'));

    Ok(ProfileLibrary {
        name: coordinate.to_string(),
        url: with_trailing_slash(sponge_maven_url.as_str()),
        downloads: Some(ProfileLibraryDownloads {
            artifact: Some(ProfileLibraryArtifact {
                path: artifact_path,
                url: artifact_url,
                size: None,
                sha1: None,
            }),
            classifiers: None,
        }),
        natives: None,
        extract: None,
    })
}

fn is_rift_loader_library(library_name: &str) -> bool {
    library_name.starts_with("org.dimdev:rift:")
        || library_name.starts_with("com.github.Chocohead:Rift:")
}
