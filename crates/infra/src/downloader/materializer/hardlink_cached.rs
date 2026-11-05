use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::{
    Materializer, NoCachedMaterializer, StagedDownload, cleanup_file, create_temp_output_file,
    replace_file,
};
use crate::downloader::{task::DownloadTask, validation::target_matches_task};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct HardlinkCachedMaterializer {
    cache_root: PathBuf,
}

impl HardlinkCachedMaterializer {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
        }
    }

    fn cache_path_for_task(&self, task: &DownloadTask) -> Result<PathBuf> {
        let sha1 = task.sha1.as_ref().ok_or(Error::HardlinkCacheSha1Missing)?;
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

        if tokio::fs::metadata(&task.path).await.is_ok() {
            tokio::fs::remove_file(&task.path).await?;
        }

        tokio::fs::hard_link(cache_path, &task.path).await?;
        Ok(())
    }
}

#[async_trait]
impl Materializer for HardlinkCachedMaterializer {
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
            return NoCachedMaterializer.create_staging(task).await;
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
            Err(_) => return NoCachedMaterializer.commit(staged, task).await,
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
