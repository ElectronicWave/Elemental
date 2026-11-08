use std::{path::PathBuf, time::Duration};

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

use super::task::{SessionId, TaskId};
use crate::error::{Error, Result};

mod failure;
mod progress;

pub use failure::{DownloadFailure, DownloadPhase};
pub use progress::{ActiveTask, DownloadProgress};

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
