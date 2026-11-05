use std::time::Instant;

use super::super::event::{TaskInfo, TaskSnapshot};
use super::super::report::{TaskOutcome, TaskStatus};
use super::super::task::{DownloadTask, TaskId};

#[derive(Debug, Clone)]
pub(super) struct TaskRecord {
    pub(super) task_id: TaskId,
    pub(super) expected_bytes: Option<u64>,
    pub(super) received_bytes: u64,
    pub(super) last_progress_event: Instant,
    pub(super) status: TaskStatus,
    group: Option<String>,
    url: String,
    path: std::path::PathBuf,
}

impl TaskRecord {
    pub(super) fn new(
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
        now: Instant,
    ) -> Self {
        Self {
            task_id,
            group,
            url: task.url.clone(),
            path: task.path.clone(),
            expected_bytes: task.expected_size,
            received_bytes: 0,
            last_progress_event: now,
            status: TaskStatus::Queued,
        }
    }

    pub(super) fn info(&self) -> TaskInfo {
        TaskInfo {
            task_id: self.task_id.clone(),
            group: self.group.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            expected_bytes: self.expected_bytes,
        }
    }

    pub(super) fn outcome(&self) -> TaskOutcome {
        TaskOutcome {
            task_id: self.task_id.clone(),
            group: self.group.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            status: match &self.status {
                TaskStatus::Queued | TaskStatus::Active => TaskStatus::Cancelled,
                status => status.clone(),
            },
        }
    }

    pub(super) fn snapshot(&self) -> TaskSnapshot {
        TaskSnapshot {
            task_id: self.task_id.clone(),
            group: self.group.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            expected_bytes: self.expected_bytes,
            received_bytes: self.received_bytes,
            status: self.status.clone(),
        }
    }
}
