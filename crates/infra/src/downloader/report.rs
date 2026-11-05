use std::{fmt, path::PathBuf, time::Duration};

use super::{task::SessionId, tracking::TaskId};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DownloadPhase {
    Prepare,
    Connect,
    Status,
    Download,
    Verify,
    Commit,
}

impl fmt::Display for DownloadPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Prepare => "prepare",
            Self::Connect => "connect",
            Self::Status => "status",
            Self::Download => "download",
            Self::Verify => "verify",
            Self::Commit => "commit",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub enum TaskStatus {
    Downloaded { bytes: u64, duration: Duration },
    Skipped { bytes: u64 },
    Failed(DownloadFailure),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOutcome {
    pub task_id: TaskId,
    pub group: Option<String>,
    pub url: String,
    pub path: PathBuf,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadReport {
    pub session_id: SessionId,
    pub session_name: Option<String>,
    pub outcomes: Vec<TaskOutcome>,
    pub downloaded_bytes: u64,
    pub skipped_bytes: u64,
    pub total_bytes: Option<u64>,
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
                TaskStatus::Failed(failure) => Some(failure),
                TaskStatus::Downloaded { .. }
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
                TaskStatus::Downloaded { .. }
                | TaskStatus::Skipped { .. }
                | TaskStatus::Failed(_) => None,
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
pub struct DownloadProgress {
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_skipped: usize,
    pub tasks_cancelled: usize,
    pub pending: usize,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_second: u64,
    pub eta: Option<Duration>,
}
