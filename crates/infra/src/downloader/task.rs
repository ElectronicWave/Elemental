use anyhow::{Result, bail};
use std::{num::NonZeroUsize, path::PathBuf};

pub type SessionId = u64;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadTask {
    pub url: String,
    pub path: PathBuf,
    pub expected_size: Option<u64>,
    pub sha1: Option<String>,
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
pub enum DownloadExecutionPolicy {
    ServiceDefault,
    Custom {
        parallelism: usize,
        rate_limit: DownloadRateLimit,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadSessionRequest {
    pub name: Option<String>,
    pub execution_policy: DownloadExecutionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlan {
    pub session: DownloadSessionRequest,
    pub tasks: Vec<DownloadTask>,
}

impl DownloadSessionRequest {
    pub fn new(name: Option<String>, execution_policy: DownloadExecutionPolicy) -> Result<Self> {
        let request = Self {
            name,
            execution_policy,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn named(
        name: impl Into<String>,
        execution_policy: DownloadExecutionPolicy,
    ) -> Result<Self> {
        Self::new(Some(name.into()), execution_policy)
    }

    pub fn unnamed(execution_policy: DownloadExecutionPolicy) -> Result<Self> {
        Self::new(None, execution_policy)
    }

    pub fn effective_parallelism(&self, default_parallelism: usize) -> usize {
        match self.execution_policy {
            DownloadExecutionPolicy::ServiceDefault => default_parallelism,
            DownloadExecutionPolicy::Custom { parallelism, .. } => parallelism,
        }
    }

    pub fn effective_rate_limit(&self) -> DownloadRateLimit {
        match self.execution_policy {
            DownloadExecutionPolicy::ServiceDefault => DownloadRateLimit::Unlimited,
            DownloadExecutionPolicy::Custom { rate_limit, .. } => rate_limit,
        }
    }

    fn validate(&self) -> Result<()> {
        if let DownloadExecutionPolicy::Custom { parallelism, .. } = self.execution_policy
            && parallelism == 0
        {
            bail!("download session parallelism must be greater than zero");
        }

        Ok(())
    }
}

impl DownloadPlan {
    pub fn new(session: DownloadSessionRequest, tasks: Vec<DownloadTask>) -> Self {
        Self { session, tasks }
    }

    pub fn named(
        name: impl Into<String>,
        execution_policy: DownloadExecutionPolicy,
        tasks: Vec<DownloadTask>,
    ) -> Result<Self> {
        Ok(Self {
            session: DownloadSessionRequest::named(name, execution_policy)?,
            tasks,
        })
    }
}

impl DownloadTask {
    pub fn new(
        url: impl Into<String>,
        path: impl Into<PathBuf>,
        expected_size: Option<u64>,
        sha1: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            path: path.into(),
            expected_size,
            sha1,
        }
    }
}
