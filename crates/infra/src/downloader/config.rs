use std::{num::NonZeroUsize, time::Duration};

use reqwest::ClientBuilder;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRate(NonZeroUsize);

impl ByteRate {
    const KIB: NonZeroUsize = NonZeroUsize::new(1024).expect("1024 must be non-zero");

    pub fn bytes_per_sec(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub fn kib_per_sec(value: NonZeroUsize) -> Result<Self> {
        value
            .checked_mul(Self::KIB)
            .map(Self::bytes_per_sec)
            .ok_or(Error::ByteRateOverflow { unit: "KiB/s" })
    }

    pub fn mib_per_sec(value: NonZeroUsize) -> Result<Self> {
        value
            .checked_mul(Self::KIB)
            .and_then(|value| value.checked_mul(Self::KIB))
            .map(Self::bytes_per_sec)
            .ok_or(Error::ByteRateOverflow { unit: "MiB/s" })
    }

    pub fn as_bytes_per_sec(self) -> usize {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadRateLimit {
    #[default]
    Unlimited,
    Limited(ByteRate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.max_attempts == 0 || self.max_backoff < self.initial_backoff {
            return Err(Error::InvalidRetryPolicy);
        }

        Ok(())
    }

    pub(crate) fn backoff(&self, retry_index: usize) -> Duration {
        let exponent = u32::try_from(retry_index.saturating_sub(1))
            .unwrap_or(u32::MAX)
            .min(31);
        self.initial_backoff
            .saturating_mul(2u32.saturating_pow(exponent))
            .min(self.max_backoff)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(8),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FailurePolicy {
    /// Keep downloading queued tasks and accept new submissions.
    #[default]
    Continue,
    /// Let in-flight tasks finish, cancel queued tasks, and reject new submissions.
    FinishInFlight,
    /// Cancel in-flight and queued tasks immediately and reject new submissions.
    CancelAll,
}

impl FailurePolicy {
    pub(crate) fn stops_scheduling(self) -> bool {
        !matches!(self, Self::Continue)
    }

    pub(crate) fn cancels_in_flight(self) -> bool {
        matches!(self, Self::CancelAll)
    }
}

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
