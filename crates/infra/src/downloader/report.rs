use std::{path::PathBuf, time::Duration};

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

use super::task::SessionId;
pub use crate::error::{DownloadFailure, DownloadPhase};
use crate::error::{Error, Result, TaskId};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ActiveTask {
    pub task_id: TaskId,
    pub group: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DownloadProgress {
    /// Tasks accepted by the session so far. Updated once per submitted batch.
    pub tasks_accepted: usize,
    /// Tasks that are fully handled, including downloaded and skipped tasks.
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
    /// Last group seen in task order, retained after it becomes inactive.
    pub last_group: Option<String>,
    /// Active tasks in enqueue order.
    pub active_tasks: Vec<ActiveTask>,
}

impl DownloadProgress {
    /// Tasks that reached a terminal state, including failed and cancelled ones.
    pub fn tasks_finished(&self) -> usize {
        self.tasks_completed
            .saturating_add(self.tasks_failed)
            .saturating_add(self.tasks_cancelled)
    }

    /// Bytes already available locally, whether downloaded or skipped.
    pub fn ready_bytes(&self) -> u64 {
        self.bytes_downloaded.saturating_add(self.bytes_skipped)
    }
}

#[cfg(feature = "serde")]
fn serialize_optional_duration_millis<S>(
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

impl TaskStatus {
    pub(crate) fn failure(&self) -> Option<&DownloadFailure> {
        match self {
            Self::Failed { failure } => Some(failure),
            _ => None,
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    pub(crate) fn finalized(self) -> Self {
        match self {
            Self::Queued | Self::Active => Self::Cancelled,
            status => status,
        }
    }
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
            .filter_map(|outcome| outcome.status.failure())
    }

    pub fn retryable_failures(&self) -> impl Iterator<Item = &DownloadFailure> {
        self.failures().filter(|failure| failure.retryable)
    }

    pub fn cancelled_task_ids(&self) -> impl Iterator<Item = &TaskId> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status.is_cancelled())
            .map(|outcome| &outcome.task_id)
    }

    pub fn is_success(&self) -> bool {
        self.failures().next().is_none() && self.cancelled_task_ids().next().is_none()
    }

    /// Return an error if any task failed or was cancelled.
    pub fn success(&self) -> Result<()> {
        let session = self
            .session_name
            .clone()
            .unwrap_or_else(|| self.session_id.to_string());
        let failures: Vec<DownloadFailure> = self.failures().cloned().collect();
        let cancelled: Vec<TaskId> = self.cancelled_task_ids().copied().collect();

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
