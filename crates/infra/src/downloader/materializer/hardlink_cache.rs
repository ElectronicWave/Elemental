use async_trait::async_trait;
use std::path::PathBuf;

use super::{Materializer, StagedDownload, UncachedMaterializer, fs::replace_file};
use crate::downloader::{task::DownloadTask, validation::DownloadValidation};
use crate::error::{Error, Result};

mod install;
mod layout;

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
}

#[async_trait]
impl Materializer for HardlinkCacheMaterializer {
    async fn try_reuse(&self, task: &DownloadTask) -> Result<bool> {
        let validation = DownloadValidation::from_task(task);
        if validation.matches(&task.path).await? {
            return Ok(true);
        }

        let cache_path = match self.cache_path_for_task(task) {
            Ok(cache_path) => cache_path,
            Err(Error::HardlinkCacheSha1Missing) => return Ok(false),
            Err(error) => return Err(error),
        };

        if !validation.matches(&cache_path).await? {
            return Ok(false);
        }

        self.materialize_from_cache(task, &cache_path).await?;
        Ok(true)
    }

    fn staging_path(&self, task: &DownloadTask) -> Result<PathBuf> {
        match self.cache_path_for_task(task) {
            Ok(cache_path) => Ok(cache_path),
            Err(Error::HardlinkCacheSha1Missing) => Ok(task.path.clone()),
            Err(error) => Err(error),
        }
    }

    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()> {
        let cache_path = match self.cache_path_for_task(task) {
            Ok(cache_path) => cache_path,
            Err(Error::HardlinkCacheSha1Missing) => {
                return UncachedMaterializer.commit(staged, task).await;
            }
            Err(error) => return Err(error),
        };

        replace_file(&staged.path, &cache_path).await?;
        self.materialize_from_cache(task, &cache_path).await
    }
}
