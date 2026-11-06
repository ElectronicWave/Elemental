use std::{fmt, path::PathBuf, time::Duration};

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

use super::task::{DownloadTask, SessionId, TaskId};
use crate::error::{Error, Result};

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
    pub(crate) fn new(
        task_id: &TaskId,
        task: &DownloadTask,
        group: Option<String>,
        phase: DownloadPhase,
        http_status: Option<u16>,
        retryable: bool,
        message: String,
    ) -> Self {
        Self {
            task_id: task_id.clone(),
            group,
            url: task.url().to_owned(),
            path: task.path().to_path_buf(),
            phase,
            http_status,
            retryable,
            message,
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "status", rename_all = "snake_case"))]
pub enum TaskStatus {
    Queued,
    Active,
    Downloaded {
        bytes: u64,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_duration_millis"))]
        duration: Duration,
    },
    Skipped {
        bytes: u64,
    },
    Failed {
        failure: DownloadFailure,
    },
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TaskOutcome {
    pub task_id: TaskId,
    pub group: Option<String>,
    pub url: String,
    pub path: PathBuf,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DownloadReport {
    pub session_id: SessionId,
    pub session_name: Option<String>,
    pub outcomes: Vec<TaskOutcome>,
    pub downloaded_bytes: u64,
    pub skipped_bytes: u64,
    pub expected_bytes: Option<u64>,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_duration_millis"))]
    pub elapsed: Duration,
}

impl DownloadReport {
    pub fn outcomes_in_group<'a>(
        &'a self,
        group: &'a str,
    ) -> impl Iterator<Item = &'a TaskOutcome> + 'a {
        self.outcomes
            .iter()
            .filter(move |outcome| outcome.group.as_deref() == Some(group))
    }

    pub fn failures(&self) -> impl Iterator<Item = &DownloadFailure> {
        self.outcomes
            .iter()
            .filter_map(|outcome| match &outcome.status {
                TaskStatus::Failed { failure } => Some(failure),
                TaskStatus::Queued
                | TaskStatus::Active
                | TaskStatus::Downloaded { .. }
                | TaskStatus::Skipped { .. }
                | TaskStatus::Cancelled => None,
            })
    }

    pub fn retryable_failures(&self) -> impl Iterator<Item = &DownloadFailure> {
        self.failures().filter(|failure| failure.retryable)
    }

    pub fn cancelled_task_ids(&self) -> impl Iterator<Item = &TaskId> {
        self.outcomes
            .iter()
            .filter_map(|outcome| match outcome.status {
                TaskStatus::Cancelled => Some(&outcome.task_id),
                TaskStatus::Queued
                | TaskStatus::Active
                | TaskStatus::Downloaded { .. }
                | TaskStatus::Skipped { .. }
                | TaskStatus::Failed { .. } => None,
            })
    }

    pub fn is_success(&self) -> bool {
        self.failures().next().is_none() && self.cancelled_task_ids().next().is_none()
    }

    pub fn success(&self) -> Result<()> {
        let session = self
            .session_name
            .clone()
            .unwrap_or_else(|| self.session_id.to_string());
        let failures: Vec<DownloadFailure> = self.failures().cloned().collect();
        let cancelled: Vec<TaskId> = self.cancelled_task_ids().cloned().collect();

        if !failures.is_empty() {
            return Err(Error::SessionFailed { session, failures });
        }

        if !cancelled.is_empty() {
            return Err(Error::SessionCancelled {
                session,
                tasks: cancelled,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DownloadProgress {
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_skipped: usize,
    pub tasks_cancelled: usize,
    pub tasks_pending: usize,
    /// Bytes successfully written to their final destinations.
    pub bytes_downloaded: u64,
    /// Bytes received from the network, including bytes from retried attempts.
    pub bytes_received: u64,
    /// Bytes that did not need to be downloaded.
    pub bytes_skipped: u64,
    pub expected_bytes: Option<u64>,
    pub bytes_per_second: u64,
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_optional_duration_millis")
    )]
    pub eta: Option<Duration>,
}

#[cfg(feature = "serde")]
pub(crate) fn serialize_duration_millis<S>(
    duration: &Duration,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(feature = "serde")]
pub(crate) fn serialize_optional_duration_millis<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match duration {
        Some(duration) => serialize_duration_millis(duration, serializer),
        None => serializer.serialize_none(),
    }
}
