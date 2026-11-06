use std::sync::Arc;

use super::{DownloadSession, DownloadSessionOptions};
use crate::downloader::core::ElementalDownloader;
use crate::downloader::policy::{FailurePolicy, RetryPolicy};
use crate::downloader::rate::DownloadRateLimit;
use crate::error::Result;

#[derive(Debug)]
pub struct DownloadSessionBuilder {
    downloader: Arc<ElementalDownloader>,
    name: String,
    options: DownloadSessionOptions,
}

impl DownloadSessionBuilder {
    pub(crate) fn new(downloader: Arc<ElementalDownloader>, name: String) -> Self {
        Self {
            downloader,
            name,
            options: DownloadSessionOptions::default(),
        }
    }

    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.options.parallelism = Some(parallelism);
        self
    }

    pub fn retry(mut self, retry_policy: RetryPolicy) -> Self {
        self.options.retry_policy = retry_policy;
        self
    }

    pub fn failure_policy(mut self, failure_policy: FailurePolicy) -> Self {
        self.options.failure_policy = failure_policy;
        self
    }

    pub fn rate_limit(mut self, rate_limit: DownloadRateLimit) -> Self {
        self.options.rate_limit = Some(rate_limit);
        self
    }

    pub async fn open(self) -> Result<DownloadSession> {
        self.downloader
            .create_session(Some(self.name), self.options)
            .await
    }
}
