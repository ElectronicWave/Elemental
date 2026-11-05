use async_trait::async_trait;
use std::{fmt, path::PathBuf};

use super::task::DownloadTask;
use crate::error::Result;

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
    async fn create_staging(&self, task: &DownloadTask) -> Result<StagedDownload>;
    async fn commit(&self, staged: StagedDownload, task: &DownloadTask) -> Result<()>;
    async fn abort(&self, staged: StagedDownload) -> Result<()>;
}
