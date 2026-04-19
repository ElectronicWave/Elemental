use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{fs::OpenOptions, fs::create_dir_all};

use super::task::DownloadTask;

mod hardlink_cached;
mod local_fs;

pub use hardlink_cached::HardlinkCachedStorage;
pub use local_fs::LocalFsStorage;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct StagedDownload {
    pub path: PathBuf,
    pub file: Option<tokio::fs::File>,
}

#[async_trait]
pub trait DownloadStorage: fmt::Debug + Send + Sync {
    async fn resolve(&self, task: &DownloadTask) -> Result<bool>;
    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload>;
    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()>;
    async fn abort(&self, staged: StagedDownload) -> Result<()>;
}

pub(super) async fn create_temp_output_file(
    parent: &Path,
    file_name: String,
) -> Result<StagedDownload> {
    create_dir_all(parent).await?;

    for _ in 0..32u32 {
        let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_path =
            parent.join(format!(".{}.part.{}.{}", file_name, std::process::id(), suffix));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .await
        {
            Ok(file) => {
                return Ok(StagedDownload {
                    path: temp_path,
                    file: Some(file),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    bail!(
        "failed to allocate temporary download file under '{}'",
        parent.display()
    )
}

pub(super) async fn replace_file(source_path: &Path, target_path: &Path) -> Result<()> {
    create_dir_all(
        target_path
            .parent()
            .context("Can't get target parent directory")?,
    )
    .await?;

    if tokio::fs::metadata(target_path).await.is_ok() {
        tokio::fs::remove_file(target_path).await?;
    }

    tokio::fs::rename(source_path, target_path).await?;
    Ok(())
}

pub(super) async fn cleanup_file(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}
