use super::{task::SessionId, tracking::TaskId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskExecutionFailure {
    pub task_id: TaskId,
    pub error: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionExecutionReport {
    pub session_id: SessionId,
    pub session_name: Option<String>,
    pub accepted: usize,
    pub enqueued: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub pending: usize,
    pub is_closed: bool,
    pub failures: Vec<TaskExecutionFailure>,
    pub cancelled_task_ids: Vec<TaskId>,
}
