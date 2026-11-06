use std::fmt;

use reqwest::header::HeaderMap;

use crate::downloader::header::HeaderNames;
use crate::downloader::policy::{FailurePolicy, RetryPolicy};
use crate::downloader::rate::DownloadRateLimit;
use crate::error::{Error, Result};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DownloadSessionOptions {
    pub parallelism: Option<usize>,
    pub rate_limit: Option<DownloadRateLimit>,
    pub retry_policy: RetryPolicy,
    pub failure_policy: FailurePolicy,
    pub headers: HeaderMap,
}

impl fmt::Debug for DownloadSessionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DownloadSessionOptions")
            .field("parallelism", &self.parallelism)
            .field("rate_limit", &self.rate_limit)
            .field("retry_policy", &self.retry_policy)
            .field("failure_policy", &self.failure_policy)
            .field("headers", &HeaderNames(&self.headers))
            .finish()
    }
}

impl DownloadSessionOptions {
    pub(crate) fn effective_parallelism(&self, default_parallelism: usize) -> usize {
        self.parallelism.unwrap_or(default_parallelism)
    }

    pub(crate) fn effective_rate_limit(&self) -> DownloadRateLimit {
        self.rate_limit.unwrap_or(DownloadRateLimit::Unlimited)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(parallelism) = self.parallelism
            && parallelism == 0
        {
            return Err(Error::InvalidSessionParallelism);
        }

        self.retry_policy.validate()?;
        Ok(())
    }
}

impl Default for DownloadSessionOptions {
    fn default() -> Self {
        Self {
            parallelism: None,
            rate_limit: None,
            retry_policy: RetryPolicy::default(),
            failure_policy: FailurePolicy::Continue,
            headers: HeaderMap::new(),
        }
    }
}
