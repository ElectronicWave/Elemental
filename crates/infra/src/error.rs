use std::{error::Error as StdError, fmt, io, path::PathBuf};

#[cfg(feature = "serde")]
use serde::Serialize;

pub type SessionId = u64;
pub type TaskId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DownloadPhase {
    Prepare,
    Connect,
    HttpStatus,
    Download,
    Verify,
    Commit,
}

impl fmt::Display for DownloadPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Prepare => "prepare",
            Self::Connect => "connect",
            Self::HttpStatus => "http_status",
            Self::Download => "download",
            Self::Verify => "verify",
            Self::Commit => "commit",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DownloadFailure {
    pub task_id: TaskId,
    pub group: Option<String>,
    pub url: String,
    pub path: PathBuf,
    pub phase: DownloadPhase,
    pub http_status: Option<u16>,
    pub retryable: bool,
    pub message: String,
}

impl DownloadFailure {
    pub(crate) fn for_task(
        task_id: TaskId,
        group: Option<String>,
        url: String,
        path: PathBuf,
        phase: DownloadPhase,
        message: String,
    ) -> Self {
        Self {
            task_id,
            group,
            url,
            path,
            phase,
            http_status: None,
            retryable: false,
            message,
        }
    }

    pub(crate) fn with_http_status(mut self, http_status: Option<u16>) -> Self {
        self.http_status = http_status;
        self
    }

    pub(crate) fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn summary(&self) -> String {
        let group = self
            .group
            .as_deref()
            .map(|group| format!("{group}/"))
            .unwrap_or_default();
        match self.http_status {
            Some(status) => format!("[{group}{}] HTTP {status}: {}", self.phase, self.message),
            None => format!("[{group}{}] {}", self.phase, self.message),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("failed to read ZIP archive: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("ZIP symlink '{entry}' points outside the destination directory: {target}")]
    ZipSymlinkEscapesDestination { entry: String, target: String },

    #[error("archive entry is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("blocking task failed: {0}")]
    BlockingJoin(#[from] tokio::task::JoinError),

    #[error("downloader max_concurrent_downloads must be greater than zero")]
    InvalidMaxConcurrentDownloads,

    #[error("session parallelism must be greater than zero")]
    InvalidSessionParallelism,

    #[error("downloader session_queue_capacity must be greater than zero")]
    InvalidSessionQueueCapacity,

    #[error("downloader retry policy requires max_attempts > 0 and max_backoff >= initial_backoff")]
    InvalidRetryPolicy,

    #[error("downloader read_timeout must be greater than zero")]
    InvalidReadTimeout,

    #[error("downloader {unit} rate limit is too large")]
    ByteRateOverflow { unit: &'static str },

    #[error("download task URL '{url}' is invalid: {reason}")]
    InvalidTaskUrl { url: String, reason: String },

    #[error("download task path '{}' is invalid: {reason}", path.display())]
    InvalidTaskPath { path: PathBuf, reason: &'static str },

    #[error("path '{path}' is not a safe relative path: {reason}")]
    UnsafeRelativePath { path: String, reason: &'static str },

    #[error("download task sha1 '{sha1}' must be 40 hexadecimal characters")]
    InvalidSha1 { sha1: String },

    #[error("download header name '{name}' is invalid: {reason}")]
    InvalidHeaderName { name: String, reason: String },

    #[error("download header value for '{name}' is invalid: {reason}")]
    InvalidHeaderValue { name: String, reason: String },

    #[error("download session '{session_id}' is closed")]
    SessionClosed { session_id: u64 },

    #[error("download session '{session_id}' not found")]
    SessionNotFound { session_id: u64 },

    #[error("download session id '{session_id}' conflicts with an existing session")]
    SessionIdConflict { session_id: u64 },

    #[error(
        "download session '{session_id}' is not accepting new tasks because an earlier task failed"
    )]
    SessionNotAcceptingTasks { session_id: u64 },

    #[error("download session is closed")]
    SessionSenderClosed,

    #[error("failed to enqueue task '{task_id}': {source}")]
    EnqueueTask {
        task_id: String,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("staged download is missing its file handle")]
    StagedFileMissing,

    #[error("download session '{session}' failed:\n{}", format_failures(failures))]
    SessionFailed {
        session: String,
        failures: Vec<DownloadFailure>,
    },

    #[error(
        "download session '{session}' was cancelled:\n{}",
        format_task_ids(tasks)
    )]
    SessionCancelled { session: String, tasks: Vec<TaskId> },

    #[error(
        "downloaded file size mismatch for '{}': expected {expected}, got {actual}",
        path.display()
    )]
    SizeMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },

    #[error(
        "downloaded file sha1 mismatch for '{}': expected {expected}, got {actual}",
        path.display()
    )]
    Sha1Mismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error(
        "failed to allocate temporary download file under '{}'",
        directory.display()
    )]
    TempFileCreation { directory: PathBuf },

    #[error("path '{}' has no {component}", path.display())]
    MissingPathComponent {
        path: PathBuf,
        component: &'static str,
    },

    #[error("hardlink cache requires task sha1")]
    HardlinkCacheSha1Missing,

    #[error("task sha1 '{sha1}' is too short for cache path")]
    Sha1TooShort { sha1: String },

    #[error("failed to shrink downloader concurrency: {0}")]
    ConcurrencyShrink(#[source] tokio::sync::AcquireError),

    #[error("failed to acquire downloader concurrency permit: {0}")]
    ConcurrencyAcquire(#[source] tokio::sync::AcquireError),
}

pub type Result<T> = std::result::Result<T, Error>;

fn format_failures(failures: &[DownloadFailure]) -> String {
    failures
        .iter()
        .map(|failure| format!("{}: {}", failure.task_id, failure.summary()))
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_task_ids(tasks: &[TaskId]) -> String {
    tasks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n")
}
