use async_trait::async_trait;
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

use super::{
    Materializer, StagedDownload, UncachedMaterializer,
    fs::{cleanup_file, create_temp_output_file, next_temp_path, replace_file},
};
use crate::downloader::{
    task::{DownloadTask, validate_sha1},
    validation::target_matches_task,
};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct HardlinkCacheMaterializer {
    cache_root: PathBuf,
}

impl HardlinkCacheMaterializer {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
        }
    }

    fn cache_path_for_task(&self, task: &DownloadTask) -> Result<PathBuf> {
        let raw_sha1 = task.sha1.as_ref().ok_or(Error::HardlinkCacheSha1Missing)?;
        validate_sha1(raw_sha1)?;
        let sha1 = raw_sha1.to_ascii_lowercase();
        let prefix = sha1
            .get(0..2)
            .ok_or_else(|| Error::Sha1TooShort { sha1: sha1.clone() })?;
        Ok(self.cache_root.join(prefix).join(sha1))
    }

    async fn materialize_from_cache(&self, task: &DownloadTask, cache_path: &Path) -> Result<()> {
        let parent = task
            .path
            .parent()
            .ok_or_else(|| Error::MissingPathComponent {
                path: task.path.clone(),
                component: "target parent directory",
            })?;
        tokio::fs::create_dir_all(parent).await?;

        let file_name = task
            .path
            .file_name()
            .ok_or_else(|| Error::MissingPathComponent {
                path: task.path.clone(),
                component: "target file name",
            })?
            .to_string_lossy()
            .to_string();

        for _ in 0..32 {
            let temp_path = next_temp_path(parent, &file_name);
            match tokio::fs::hard_link(cache_path, &temp_path).await {
                Ok(()) => {
                    let result = replace_file(&temp_path, &task.path).await;
                    if result.is_err() {
                        cleanup_file(&temp_path).await;
                    }
                    return result;
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) if should_fallback_to_copy(&error) => {
                    return copy_file_into_place(cache_path, &task.path, parent, &file_name).await;
                }
                Err(error) => return Err(error.into()),
            }
        }

        Err(Error::TempFileCreation {
            directory: parent.to_path_buf(),
        })
    }
}

fn should_fallback_to_copy(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::CrossesDevices | ErrorKind::Unsupported | ErrorKind::PermissionDenied
    )
}

async fn copy_file_into_place(
    cache_path: &Path,
    target_path: &Path,
    parent: &Path,
    file_name: &str,
) -> Result<()> {
    let StagedDownload {
        path: temp_path,
        file,
    } = create_temp_output_file(parent, file_name.to_owned()).await?;

    let result = copy_file_contents(cache_path, file, &temp_path, target_path).await;
    if result.is_err() {
        cleanup_file(&temp_path).await;
    }
    result
}

async fn copy_file_contents(
    cache_path: &Path,
    file: Option<tokio::fs::File>,
    temp_path: &Path,
    target_path: &Path,
) -> Result<()> {
    let mut source = tokio::fs::File::open(cache_path).await?;
    let mut destination = file.ok_or(Error::StagedFileMissing)?;
    tokio::io::copy(&mut source, &mut destination).await?;
    destination.flush().await?;
    drop(destination);
    drop(source);
    replace_file(temp_path, target_path).await
}

#[async_trait]
impl Materializer for HardlinkCacheMaterializer {
    async fn try_reuse(&self, task: &DownloadTask) -> Result<bool> {
        if target_matches_task(&task.path, task).await? {
            return Ok(true);
        }

        let cache_path = match self.cache_path_for_task(task) {
            Ok(cache_path) => cache_path,
            Err(Error::HardlinkCacheSha1Missing) => return Ok(false),
            Err(error) => return Err(error),
        };

        if !target_matches_task(&cache_path, task).await? {
            return Ok(false);
        }

        self.materialize_from_cache(task, &cache_path).await?;
        Ok(true)
    }

    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload> {
        if task.sha1.is_none() {
            return UncachedMaterializer.create_staging(task).await;
        }

        let cache_path = self.cache_path_for_task(task)?;
        let parent = cache_path
            .parent()
            .ok_or_else(|| Error::MissingPathComponent {
                path: cache_path.clone(),
                component: "cache parent directory",
            })?;
        let file_name = cache_path
            .file_name()
            .ok_or_else(|| Error::MissingPathComponent {
                path: cache_path.clone(),
                component: "cache file name",
            })?
            .to_string_lossy()
            .to_string();
        create_temp_output_file(parent, file_name).await
    }

    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()> {
        let cache_path = match self.cache_path_for_task(task) {
            Ok(cache_path) => cache_path,
            Err(Error::HardlinkCacheSha1Missing) => {
                return UncachedMaterializer.commit(staged, task).await;
            }
            Err(error) => return Err(error),
        };

        if target_matches_task(&cache_path, task).await? {
            cleanup_file(&staged.path).await;
            self.materialize_from_cache(task, &cache_path).await?;
            return Ok(());
        }

        replace_file(&staged.path, &cache_path).await?;
        self.materialize_from_cache(task, &cache_path).await
    }

    async fn abort(&self, staged: StagedDownload) -> Result<()> {
        cleanup_file(&staged.path).await;
        Ok(())
    }
}
