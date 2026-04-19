use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{DownloadStorage, StagedDownload, create_temp_output_file, replace_file};
use crate::downloader::{task::DownloadTask, validation::target_matches_task};

#[derive(Debug, Clone, Default)]
pub struct LocalFsStorage;

#[async_trait]
impl DownloadStorage for LocalFsStorage {
    async fn resolve(&self, task: &DownloadTask) -> Result<bool> {
        target_matches_task(&task.path, task).await
    }

    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload> {
        create_temp_output_file(
            task.path
                .parent()
                .context("Can't get target parent directory")?,
            task.path
                .file_name()
                .context("Can't get target file name")?
                .to_string_lossy()
                .to_string(),
        )
        .await
    }

    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()> {
        replace_file(&staged.path, &task.path).await
    }

    async fn abort(&self, staged: StagedDownload) -> Result<()> {
        super::cleanup_file(&staged.path).await;
        Ok(())
    }
}
