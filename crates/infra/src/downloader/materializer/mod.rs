use async_trait::async_trait;
use std::{fmt, path::PathBuf};

use super::task::DownloadTask;
use crate::error::{Error, Result};

mod fs;
mod hardlink_cache;
mod uncached;

pub use hardlink_cache::HardlinkCacheMaterializer;
pub use uncached::UncachedMaterializer;

#[derive(Debug)]
pub struct StagedDownload {
    pub path: PathBuf,
    pub file: Option<tokio::fs::File>,
}

#[async_trait]
pub trait Materializer: fmt::Debug + Send + Sync {
    async fn try_reuse(&self, task: &DownloadTask) -> Result<bool>;
    fn staging_path(&self, task: &DownloadTask) -> Result<PathBuf>;
    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload> {
        let staging_path = self.staging_path(task)?;
        let parent = staging_path
            .parent()
            .ok_or_else(|| Error::MissingPathComponent {
                path: staging_path.clone(),
                component: "staging parent directory",
            })?;
        let file_name = staging_path
            .file_name()
            .ok_or_else(|| Error::MissingPathComponent {
                path: staging_path.clone(),
                component: "staging file name",
            })?
            .to_string_lossy()
            .into_owned();
        fs::create_temp_output_file(parent, file_name).await
    }
    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()> {
        fs::replace_file(&staged.path, &task.path).await
    }
    async fn abort(&self, staged: StagedDownload) -> Result<()> {
        fs::cleanup_file(&staged.path).await;
        Ok(())
    }
}
