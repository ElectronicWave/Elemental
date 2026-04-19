use super::task::{DownloadTask, SessionId};

pub type TaskId = String;

#[derive(Debug, Clone)]
pub struct TrackedInfo {
    pub recv: usize,
    pub status: TrackedTaskStatus,
}

impl TrackedInfo {
    pub fn waiting() -> Self {
        Self {
            recv: 0,
            status: TrackedTaskStatus::WAITING,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TrackedTaskStatus {
    ERR(String),
    ACTIVE,
    CANCELLED,
    WAITING,
}

pub fn build_task_id(session_id: SessionId, task: &DownloadTask) -> TaskId {
    format!("{}-{}-{}", session_id, task.url, task.path.display())
}
