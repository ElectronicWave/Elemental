use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::{
    DownloadStorage, LocalFsStorage, StagedDownload, cleanup_file, create_temp_output_file,
    replace_file,
};
use crate::downloader::{task::DownloadTask, validation::target_matches_task};

#[derive(Debug, Clone)]
pub struct HardlinkCachedStorage {
    cache_root: PathBuf,
}

impl HardlinkCachedStorage {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
        }
    }

    fn cache_path_for_task(&self, task: &DownloadTask) -> Result<PathBuf> {
        let sha1 = task
            .sha1
            .as_ref()
            .context("hardlink cache requires task sha1")?;
        let prefix = sha1
            .get(0..2)
            .context("task sha1 is too short for cache path")?;
        Ok(self.cache_root.join(prefix).join(sha1))
    }

    async fn materialize_from_cache(&self, task: &DownloadTask, cache_path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(
            task.path
                .parent()
                .context("Can't get target parent directory")?,
        )
        .await?;

        if tokio::fs::metadata(&task.path).await.is_ok() {
            tokio::fs::remove_file(&task.path).await?;
        }

        tokio::fs::hard_link(cache_path, &task.path).await?;
        Ok(())
    }
}

#[async_trait]
impl DownloadStorage for HardlinkCachedStorage {
    async fn resolve(&self, task: &DownloadTask) -> Result<bool> {
        if target_matches_task(&task.path, task).await? {
            return Ok(true);
        }

        let cache_path = match self.cache_path_for_task(task) {
            Ok(cache_path) => cache_path,
            Err(_) => return Ok(false),
        };

        if !target_matches_task(&cache_path, task).await? {
            return Ok(false);
        }

        self.materialize_from_cache(task, &cache_path).await?;
        Ok(true)
    }

    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload> {
        if task.sha1.is_none() {
            return LocalFsStorage.create_staging(task).await;
        }

        let cache_path = self.cache_path_for_task(task)?;
        let parent = cache_path
            .parent()
            .context("Can't get cache parent directory")?;
        let file_name = cache_path
            .file_name()
            .context("Can't get cache file name")?
            .to_string_lossy()
            .to_string();
        create_temp_output_file(parent, file_name).await
    }

    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()> {
        let cache_path = match self.cache_path_for_task(task) {
            Ok(cache_path) => cache_path,
            Err(_) => return LocalFsStorage.commit(staged, task).await,
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
