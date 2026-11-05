use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::fs::{OpenOptions, create_dir_all};

use super::StagedDownload;
use crate::error::{Error, Result};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) async fn create_temp_output_file(
    parent: &Path,
    file_name: String,
) -> Result<StagedDownload> {
    create_dir_all(parent).await?;

    for _ in 0..32u32 {
        let temp_path = next_temp_path(parent, &file_name);
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
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    Err(Error::TempFileCreation {
        directory: parent.to_path_buf(),
    })
}

pub(super) fn next_temp_path(parent: &Path, file_name: &str) -> PathBuf {
    let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{}.part.{}.{}",
        file_name,
        std::process::id(),
        suffix
    ))
}

pub(super) async fn replace_file(source_path: &Path, target_path: &Path) -> Result<()> {
    let parent = target_path
        .parent()
        .ok_or_else(|| Error::MissingPathComponent {
            path: target_path.to_path_buf(),
            component: "target parent directory",
        })?;
    create_dir_all(parent).await?;

    match tokio::fs::rename(source_path, target_path).await {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::AlreadyExists | ErrorKind::PermissionDenied
            ) =>
        {
            // Windows does not replace an existing destination during rename.
            remove_file_if_exists(target_path).await?;
            tokio::fs::rename(source_path, target_path)
                .await
                .map_err(Error::from)
        }
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn remove_file_if_exists(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn cleanup_file(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}
