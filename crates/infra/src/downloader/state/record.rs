use std::time::Instant;

use super::super::event::{TaskInfo, TaskSnapshot};
use super::super::report::{TaskOutcome, TaskStatus};
use super::super::task::{DownloadTask, TaskId};

#[derive(Debug, Clone)]
pub(super) struct TaskRecord {
    pub(super) details: TaskInfo,
    pub(super) received_bytes: u64,
    pub(super) last_progress_event: Instant,
    pub(super) status: TaskStatus,
}

impl TaskRecord {
    pub(super) fn new(
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
        now: Instant,
    ) -> Self {
        Self {
            details: TaskInfo {
                task_id,
                group,
                url: task.url.clone(),
                path: task.path.clone(),
                expected_bytes: task.expected_size,
            },
            received_bytes: 0,
            last_progress_event: now,
            status: TaskStatus::Queued,
        }
    }

    pub(super) fn outcome(&self) -> TaskOutcome {
        TaskOutcome {
            task_id: self.details.task_id.clone(),
            group: self.details.group.clone(),
            url: self.details.url.clone(),
            path: self.details.path.clone(),
            status: match &self.status {
                TaskStatus::Queued | TaskStatus::Active => TaskStatus::Cancelled,
                status => status.clone(),
            },
        }
    }

    pub(super) fn snapshot(&self) -> TaskSnapshot {
        TaskSnapshot {
            task_id: self.details.task_id.clone(),
            group: self.details.group.clone(),
            url: self.details.url.clone(),
            path: self.details.path.clone(),
            expected_bytes: self.details.expected_bytes,
            received_bytes: self.received_bytes,
            status: self.status.clone(),
        }
    }
}
