use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use elemental_core::storage::layout::Layoutable;
use elemental_driver::families::version_json::{
    GameStorage, InstanceStorage, VersionJsonRootResource,
};
use elemental_infra::{
    SafeRelativePath,
    downloader::{DownloadSession, DownloadTask},
};
use nidhogg::authlib_injector::Artifact;
#[cfg(feature = "authlib-injector-embed")]
use nidhogg::authlib_injector::{EmbeddedArtifact, embedded};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

use super::endpoints::{AuthlibInjector, AuthlibInjectorEndpoints, AuthlibInjectorSource};

const ARTIFACT_DIRECTORY: &str = "authlib-injector";

pub(super) fn artifact_relative_path(version: &str) -> Result<SafeRelativePath> {
    Ok(SafeRelativePath::parse(format!(
        "{ARTIFACT_DIRECTORY}/authlib-injector-{version}.jar"
    ))?)
}

pub(super) fn jar_path_for_storage(storage: &GameStorage, version: &str) -> Result<PathBuf> {
    let relative = artifact_relative_path(version)?;
    let path = storage.try_get_resource(VersionJsonRootResource::Elemental(Some(relative)))?;
    Ok(std::path::absolute(path)?)
}

pub(super) fn jar_path_for_instance(instance: &InstanceStorage, version: &str) -> Result<PathBuf> {
    let relative = artifact_relative_path(version)?;
    let path =
        instance.try_get_parent_resource(VersionJsonRootResource::Elemental(Some(relative)))?;
    Ok(std::path::absolute(path)?)
}

pub(super) async fn ensure_remote(
    endpoints: &AuthlibInjectorEndpoints,
    session: &DownloadSession,
    artifact: Artifact,
    resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
) -> Result<AuthlibInjector> {
    let path = resolve_path(&artifact.version)?;
    if existing_artifact_matches(&path, &artifact.checksums.sha256).await? {
        return Ok(AuthlibInjector::new(
            path,
            artifact.version,
            AuthlibInjectorSource::Remote,
            None,
        ));
    }

    let download_url = endpoints.rewrite_download_url(&artifact.download_url)?;
    match install_remote(&path, session, &download_url, &artifact.checksums.sha256).await {
        Ok(()) => Ok(AuthlibInjector::new(
            path,
            artifact.version,
            AuthlibInjectorSource::Remote,
            None,
        )),
        Err(error) => install_embedded_fallback(resolve_path, &artifact.version, error).await,
    }
}

async fn install_remote(
    path: &Path,
    session: &DownloadSession,
    download_url: &str,
    expected_sha256: &str,
) -> Result<()> {
    ensure_parent(path).await?;
    remove_if_present(path).await?;
    session
        .submit(DownloadTask::new(download_url, path))
        .await?;
    verify_file(path, expected_sha256).await
}

async fn verify_file(path: &Path, expected: &str) -> Result<()> {
    let Some(actual) = file_sha256(path).await? else {
        return Err(Error::AuthlibInjectorMissing {
            path: path.to_owned(),
        });
    };
    if actual == expected {
        return Ok(());
    }

    remove_if_present(path).await?;
    Err(Error::AuthlibInjectorChecksum {
        path: path.to_owned(),
        expected: expected.to_owned(),
        actual,
    })
}

#[cfg(feature = "authlib-injector-embed")]
async fn install_embedded_fallback(
    resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
    requested_version: &str,
    error: Error,
) -> Result<AuthlibInjector> {
    let embedded_artifact = embedded()?;
    if embedded_artifact.artifact().version != requested_version {
        return Err(error);
    }
    let path = resolve_path(&embedded_artifact.artifact().version)?;
    install_embedded_with_reason(&path, embedded_artifact, Some(error.to_string())).await
}

#[cfg(not(feature = "authlib-injector-embed"))]
async fn install_embedded_fallback(
    _resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
    _requested_version: &str,
    error: Error,
) -> Result<AuthlibInjector> {
    Err(error)
}

#[cfg(feature = "authlib-injector-embed")]
pub(super) async fn install_embedded_with_reason(
    path: &Path,
    embedded: EmbeddedArtifact,
    fallback_reason: Option<String>,
) -> Result<AuthlibInjector> {
    ensure_parent(path).await?;
    remove_if_present(path).await?;
    tokio::fs::write(path, embedded.jar()).await?;
    let artifact = embedded.artifact();
    verify_file(path, &artifact.checksums.sha256).await?;

    Ok(AuthlibInjector::new(
        path.to_owned(),
        artifact.version.clone(),
        AuthlibInjectorSource::Embedded,
        fallback_reason,
    ))
}

pub(super) fn matches_requested_version(version: &str, build_number: u64, requested: &str) -> bool {
    version == requested || build_number.to_string() == requested
}

async fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    Ok(())
}

async fn existing_artifact_matches(path: &Path, expected: &str) -> Result<bool> {
    let Some(actual) = file_sha256(path).await? else {
        return Ok(false);
    };
    Ok(actual == expected)
}

async fn file_sha256(path: &Path) -> Result<Option<String>> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(Some(hex::encode(Sha256::digest(bytes)))),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(source.into()),
    }
}

async fn remove_if_present(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(()),
        Err(source) => Err(source.into()),
    }
}
