use std::{path::PathBuf, time::Duration};

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

use crate::downloader::task::TaskId;

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
        Some(duration) => super::serialize_duration_millis(duration, serializer),
        None => serializer.serialize_none(),
    }
}
