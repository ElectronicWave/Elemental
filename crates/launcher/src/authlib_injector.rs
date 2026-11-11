use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_infra::downloader::{DownloadTask, ElementalDownloader};
use nidhogg::authlib_injector::{Artifact, Distribution};
#[cfg(feature = "authlib-injector-embed")]
use nidhogg::authlib_injector::{ArtifactSource, EmbeddedArtifact, embedded};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const ARTIFACT_DIRECTORY: &str = ".elemental/authlib-injector";
const ARTIFACT_FILE_NAME: &str = "authlib-injector.jar";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthlibInjectorSource {
    Remote,
    #[cfg(feature = "authlib-injector-embed")]
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthlibInjector {
    path: PathBuf,
    version: String,
    source: AuthlibInjectorSource,
    fallback_reason: Option<String>,
}

impl AuthlibInjector {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn source(&self) -> AuthlibInjectorSource {
        self.source
    }

    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }

    pub fn jvm_arguments(&self, auth_server: &str) -> [String; 2] {
        [
            format!("-javaagent:{}={auth_server}", self.path.display()),
            "-Dauthlibinjector.side=client".to_owned(),
        ]
    }
}

pub(crate) async fn install(
    storage_root: &Path,
    downloader: &Arc<ElementalDownloader>,
) -> Result<AuthlibInjector> {
    let path = std::path::absolute(
        storage_root
            .join(ARTIFACT_DIRECTORY)
            .join(ARTIFACT_FILE_NAME),
    )?;

    let distribution = Distribution::official()?;
    #[cfg(feature = "authlib-injector-embed")]
    {
        match distribution.latest_or_embedded().await? {
            ArtifactSource::Latest(artifact) => install_latest(&path, downloader, artifact).await,
            ArtifactSource::Embedded(embedded) => install_embedded(&path, embedded).await,
        }
    }
    #[cfg(not(feature = "authlib-injector-embed"))]
    {
        let artifact = distribution.latest().await?;
        install_latest(&path, downloader, artifact).await
    }
}

async fn install_latest(
    path: &Path,
    downloader: &Arc<ElementalDownloader>,
    artifact: Artifact,
) -> Result<AuthlibInjector> {
    if existing_artifact_matches(path, &artifact.checksums.sha256).await? {
        return Ok(AuthlibInjector {
            path: path.to_owned(),
            version: artifact.version,
            source: AuthlibInjectorSource::Remote,
            fallback_reason: None,
        });
    }

    match install_remote(path, downloader, &artifact).await {
        Ok(()) => Ok(AuthlibInjector {
            path: path.to_owned(),
            version: artifact.version,
            source: AuthlibInjectorSource::Remote,
            fallback_reason: None,
        }),
        Err(error) => install_embedded_fallback(path, error).await,
    }
}

async fn install_remote(
    path: &Path,
    downloader: &Arc<ElementalDownloader>,
    artifact: &Artifact,
) -> Result<()> {
    ensure_parent(path).await?;
    remove_if_present(path).await?;
    download_artifact(path, downloader, artifact).await?;
    verify_file(path, &artifact.checksums.sha256).await
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
async fn install_embedded_fallback(path: &Path, error: Error) -> Result<AuthlibInjector> {
    install_embedded_with_reason(path, embedded()?, Some(error.to_string())).await
}

#[cfg(not(feature = "authlib-injector-embed"))]
async fn install_embedded_fallback(_path: &Path, error: Error) -> Result<AuthlibInjector> {
    Err(error)
}

async fn download_artifact(
    path: &Path,
    downloader: &Arc<ElementalDownloader>,
    artifact: &Artifact,
) -> Result<()> {
    let session = downloader
        .session("install authlib-injector")
        .open()
        .await?;
    session
        .submit(DownloadTask::new(&artifact.download_url, path))
        .await?;
    let report = session.finish().await?;
    report.success()?;
    Ok(())
}

#[cfg(feature = "authlib-injector-embed")]
async fn install_embedded(path: &Path, embedded: EmbeddedArtifact) -> Result<AuthlibInjector> {
    install_embedded_with_reason(path, embedded, None).await
}

#[cfg(feature = "authlib-injector-embed")]
async fn install_embedded_with_reason(
    path: &Path,
    embedded: EmbeddedArtifact,
    fallback_reason: Option<String>,
) -> Result<AuthlibInjector> {
    ensure_parent(path).await?;
    remove_if_present(path).await?;
    tokio::fs::write(path, embedded.jar()).await?;
    let artifact = embedded.artifact();
    verify_file(path, &artifact.checksums.sha256).await?;

    Ok(AuthlibInjector {
        path: path.to_owned(),
        version: artifact.version.clone(),
        source: AuthlibInjectorSource::Embedded,
        fallback_reason,
    })
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
