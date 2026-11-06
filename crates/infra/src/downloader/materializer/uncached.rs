use async_trait::async_trait;
use std::path::PathBuf;

use super::Materializer;
use crate::downloader::{task::DownloadTask, validation::DownloadValidation};
use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct UncachedMaterializer;

#[async_trait]
impl Materializer for UncachedMaterializer {
    async fn try_reuse(&self, task: &DownloadTask) -> Result<bool> {
        DownloadValidation::from_task(task)
            .matches(&task.path)
            .await
    }

    fn staging_path(&self, task: &DownloadTask) -> Result<PathBuf> {
        Ok(task.path.clone())
    }
}
