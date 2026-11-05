use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use super::policy::{FailurePolicy, RetryPolicy};
use crate::error::{Error, Result};

pub type SessionId = u64;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub(crate) url: String,
    pub(crate) path: PathBuf,
    pub(crate) expected_size: Option<u64>,
    pub(crate) sha1: Option<String>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRate(NonZeroUsize);

impl ByteRate {
    const KIB: NonZeroUsize = NonZeroUsize::new(1024).expect("1024 must be non-zero");

    pub fn bytes_per_sec(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub fn kib_per_sec(value: NonZeroUsize) -> Self {
        Self::bytes_per_sec(
            value
                .checked_mul(Self::KIB)
                .expect("KiB/s conversion overflowed when building ByteRate"),
        )
    }

    pub fn mib_per_sec(value: NonZeroUsize) -> Self {
        Self::kib_per_sec(
            value
                .checked_mul(Self::KIB)
                .expect("MiB/s conversion overflowed when building ByteRate"),
        )
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadSessionOptions {
    pub parallelism: Option<usize>,
    pub rate_limit: Option<DownloadRateLimit>,
    pub retry_policy: RetryPolicy,
    pub failure_policy: FailurePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadBatch {
    group: Option<String>,
    tasks: Vec<DownloadTask>,
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
        }
    }
}

impl DownloadBatch {
    pub fn new(tasks: Vec<DownloadTask>) -> Self {
        Self { group: None, tasks }
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub(crate) fn into_parts(self) -> (Option<String>, Vec<DownloadTask>) {
        (self.group, self.tasks)
    }
}

impl From<DownloadTask> for DownloadBatch {
    fn from(task: DownloadTask) -> Self {
        Self::new(vec![task])
    }
}

impl From<Vec<DownloadTask>> for DownloadBatch {
    fn from(tasks: Vec<DownloadTask>) -> Self {
        Self::new(tasks)
    }
}

impl DownloadTask {
    pub fn new(url: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            expected_size: None,
            sha1: None,
        }
    }

    pub fn size(mut self, expected_size: u64) -> Self {
        self.expected_size = Some(expected_size);
        self
    }

    pub fn sha1(mut self, sha1: impl Into<String>) -> Self {
        self.sha1 = Some(sha1.into());
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn expected_size(&self) -> Option<u64> {
        self.expected_size
    }

    pub fn sha1_checksum(&self) -> Option<&str> {
        self.sha1.as_deref()
    }
}
