use std::time::Duration;

use reqwest::ClientBuilder;

use super::rate::DownloadRateLimit;
use crate::error::{Error, Result};

#[derive(Debug)]
pub(super) struct ValidatedDownloaderConfig {
    pub(super) max_concurrent_downloads: usize,
    pub(super) default_session_parallelism: usize,
    pub(super) session_queue_capacity: usize,
    pub(super) connect_timeout: Duration,
    pub(super) read_timeout: Duration,
    pub(super) rate_limit: DownloadRateLimit,
}

impl ValidatedDownloaderConfig {
    pub(super) fn client(&self) -> Result<reqwest::Client> {
        ClientBuilder::new()
            .connect_timeout(self.connect_timeout)
            .read_timeout(self.read_timeout)
            .build()
            .map_err(Error::from)
    }
}

#[derive(Debug)]
pub struct ElementalDownloaderConfig {
    pub max_concurrent_downloads: usize,
    pub default_session_parallelism: usize,
    pub session_queue_capacity: usize,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub rate_limit: DownloadRateLimit,
}

impl Default for ElementalDownloaderConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 8,
            default_session_parallelism: 8,
            session_queue_capacity: build_queue_capacity(8),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            rate_limit: DownloadRateLimit::Unlimited,
        }
    }
}

pub(super) fn build_queue_capacity(worker_count: usize) -> usize {
    (worker_count.saturating_mul(32)).max(128)
}

impl TryFrom<ElementalDownloaderConfig> for ValidatedDownloaderConfig {
    type Error = Error;

    fn try_from(config: ElementalDownloaderConfig) -> Result<Self> {
        if config.max_concurrent_downloads == 0 {
            return Err(Error::InvalidMaxConcurrentDownloads);
        }

        if config.default_session_parallelism == 0 {
            return Err(Error::InvalidSessionParallelism);
        }

        if config.session_queue_capacity == 0 {
            return Err(Error::InvalidSessionQueueCapacity);
        }

        if config.read_timeout.is_zero() {
            return Err(Error::InvalidReadTimeout);
        }

        Ok(Self {
            max_concurrent_downloads: config.max_concurrent_downloads,
            default_session_parallelism: config.default_session_parallelism,
            session_queue_capacity: config.session_queue_capacity,
            connect_timeout: config.connect_timeout,
            read_timeout: config.read_timeout,
            rate_limit: config.rate_limit,
        })
    }
}
