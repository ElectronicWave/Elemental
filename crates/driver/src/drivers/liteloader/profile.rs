use std::collections::HashSet;

use elemental_infra::SafeRelativePath;
use elemental_schema::{
    fabric::{ProfileJson, ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads},
    mojang::piston::{PistonMetaArguments, PistonMetaGenericArgument},
};

use crate::{
    error::{Error, Result},
    maven::{MavenArtifactDownload, MavenNotation, MavenRepository},
};

use super::{
    manifest::{
        LiteLoaderLibraryResolution, LiteLoaderManifestLibrary, LiteLoaderRepositoryStream,
        LiteLoaderSelectedBuild,
    },
    source::LiteLoaderEndpoints,
};

pub(super) async fn build_profile_json(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    selected: &LiteLoaderSelectedBuild,
) -> Result<ProfileJson> {
    let mut libraries = Vec::new();
    libraries.push(build_loader_library(client, endpoints, selected).await?);

    for library in collect_manifest_libraries(selected) {
        libraries.push(normalize_manifest_library(client, endpoints, library).await?);
    }

    let timestamp = selected
        .build
        .timestamp
        .clone()
        .unwrap_or_else(|| "0".to_owned());

    Ok(ProfileJson {
        id: format!(
            "{}-liteloader-{}",
            selected.game_version, selected.build.version
        ),
        inherits_from: selected.game_version.clone(),
        arguments: Some(PistonMetaArguments {
            game: vec![
                PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                PistonMetaGenericArgument::Plain(selected.build.tweak_class.clone()),
            ],
            jvm: Vec::new(),
        }),
        assets: None,
        libraries,
        logging: None,
        main_class: "net.minecraft.launchwrapper.Launch".to_owned(),
        minimum_launcher_version: None,
        release_type: selected.build.stream.to_ascii_lowercase(),
        time: timestamp.clone(),
        release_time: timestamp,
    })
}

fn collect_manifest_libraries(
    selected: &LiteLoaderSelectedBuild,
) -> Vec<LiteLoaderManifestLibrary> {
    let mut seen = HashSet::new();
    let mut libraries = Vec::new();

    for library in selected
        .common_libraries
        .iter()
        .chain(selected.build.libraries.iter())
    {
        if seen.insert(library.name.clone()) {
            libraries.push(library.clone());
        }
    }

    libraries
}

async fn build_loader_library(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    selected: &LiteLoaderSelectedBuild,
) -> Result<ProfileLibrary> {
    let notation = format!("com.mumfrey:liteloader:{}", selected.build.version);
    let parsed = MavenNotation::parse(notation.as_str(), "liteloader library")?;
    let repository = rewritten_repository(
        endpoints,
        selected.repository.url.as_str(),
        "liteloader repository",
    )?;
    let artifact = if selected.repository.stream == LiteLoaderRepositoryStream::Snapshot {
        repository
            .resolve_snapshot_artifact(client, &parsed)
            .await?
    } else {
        repository.artifact_download(SafeRelativePath::parse(format!(
            "{}/{}",
            parsed.version_directory_path()?.as_str(),
            selected.build.file.as_str()
        ))?)?
    };

    Ok(profile_library_with_repository(
        notation,
        &repository,
        Some(artifact),
    ))
}

fn profile_library_with_repository(
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

async fn normalize_manifest_library(
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

fn fixed_library_repository(
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

fn rewritten_repository(
    endpoints: &LiteLoaderEndpoints,
    raw_url: &str,
    label: &str,
) -> Result<MavenRepository> {
    MavenRepository::parse(endpoints.rewrite_upstream(raw_url)?.as_str(), label)
}

fn default_snapshot_repository(
    endpoints: &LiteLoaderEndpoints,
    notation: &MavenNotation,
    library_name: &str,
) -> Result<MavenRepository> {
    if notation.group() == "org.spongepowered" {
        return MavenRepository::parse(
            endpoints.sponge_maven_url()?.as_str(),
            "Sponge Maven repository",
        );
    }

    Err(Error::LiteLoaderSnapshotRepositoryMissing {
        library: library_name.to_owned(),
    })
}
