use async_trait::async_trait;

use super::{
    Materializer, StagedDownload,
    fs::{cleanup_file, create_temp_output_file, replace_file},
};
use crate::downloader::{task::DownloadTask, validation::target_matches_task};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct UncachedMaterializer;

#[async_trait]
impl Materializer for UncachedMaterializer {
    async fn try_reuse(&self, task: &DownloadTask) -> Result<bool> {
        target_matches_task(&task.path, task).await
    }

    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload> {
        let parent = task
            .path
            .parent()
            .ok_or_else(|| Error::MissingPathComponent {
                path: task.path.clone(),
                component: "target parent directory",
            })?;
        let file_name = task
            .path
            .file_name()
            .ok_or_else(|| Error::MissingPathComponent {
                path: task.path.clone(),
                component: "target file name",
            })?
            .to_string_lossy()
            .to_string();

        create_temp_output_file(parent, file_name).await
    }

    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()> {
        replace_file(&staged.path, &task.path).await
    }

    async fn abort(&self, staged: StagedDownload) -> Result<()> {
        cleanup_file(&staged.path).await;
        Ok(())
    }
}
