use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::Serialize;
use tokio::sync::broadcast;

use super::report::{DownloadProgress, DownloadReport, TaskOutcome, TaskStatus};
use super::task::{SessionId, TaskId};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum DownloadEvent {
    SessionOpened {
        session_id: SessionId,
        session_name: Option<String>,
    },
    TaskQueued {
        session_id: SessionId,
        task: TaskInfo,
    },
    TaskStarted {
        session_id: SessionId,
        task_id: TaskId,
    },
    TaskProgress {
        session_id: SessionId,
        task_id: TaskId,
        received_bytes: u64,
        expected_bytes: Option<u64>,
    },
    TaskRetrying {
        session_id: SessionId,
        task_id: TaskId,
        failed_attempt: usize,
        delay_ms: u64,
        reason: String,
    },
    TaskFinished {
        session_id: SessionId,
        outcome: TaskOutcome,
    },
    SessionProgress {
        session_id: SessionId,
        progress: DownloadProgress,
    },
    SessionClosed {
        session_id: SessionId,
        report: DownloadReport,
    },
}

impl DownloadEvent {
    pub fn session_id(&self) -> SessionId {
        match self {
            Self::SessionOpened { session_id, .. }
            | Self::TaskQueued { session_id, .. }
            | Self::TaskStarted { session_id, .. }
            | Self::TaskProgress { session_id, .. }
            | Self::TaskRetrying { session_id, .. }
            | Self::TaskFinished { session_id, .. }
            | Self::SessionProgress { session_id, .. }
            | Self::SessionClosed { session_id, .. } => *session_id,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventStreamError {
    #[error("download event stream lagged by {0} events")]
    Lagged(u64),
    #[error("download event stream is closed")]
    Closed,
}

pub struct DownloadEventStream {
    events: EventReceiver,
}

impl DownloadEventStream {
    pub fn new(receiver: broadcast::Receiver<DownloadEvent>) -> Self {
        Self {
            events: EventReceiver::new(receiver),
        }
    }

    pub async fn recv(&mut self) -> std::result::Result<DownloadEvent, EventStreamError> {
        self.events.recv().await
    }
}

pub struct SessionEventStream {
    events: EventReceiver,
    session_id: SessionId,
    closed: bool,
}

impl SessionEventStream {
    pub fn new(receiver: broadcast::Receiver<DownloadEvent>, session_id: SessionId) -> Self {
        Self {
            events: EventReceiver::new(receiver),
            session_id,
            closed: false,
        }
    }

    pub async fn recv(&mut self) -> std::result::Result<DownloadEvent, EventStreamError> {
        if self.closed {
            return Err(EventStreamError::Closed);
        }

        loop {
            let event = self.events.recv().await?;
            if event.session_id() != self.session_id {
                continue;
            }

            if matches!(event, DownloadEvent::SessionClosed { .. }) {
                self.closed = true;
            }

            return Ok(event);
        }
    }
}

struct EventReceiver {
    receiver: broadcast::Receiver<DownloadEvent>,
}

impl EventReceiver {
    fn new(receiver: broadcast::Receiver<DownloadEvent>) -> Self {
        Self { receiver }
    }

    async fn recv(&mut self) -> std::result::Result<DownloadEvent, EventStreamError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Lagged(count)) => Err(EventStreamError::Lagged(count)),
            Err(broadcast::error::RecvError::Closed) => Err(EventStreamError::Closed),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TaskInfo {
    pub task_id: TaskId,
    pub group: Option<String>,
    pub url: String,
    pub path: PathBuf,
    pub expected_bytes: Option<u64>,
}

impl TaskInfo {
    pub(crate) fn outcome(&self, status: TaskStatus) -> TaskOutcome {
        TaskOutcome {
            task_id: self.task_id.clone(),
            group: self.group.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            status,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub session_name: Option<String>,
    pub progress: DownloadProgress,
    pub tasks: Vec<TaskSnapshot>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TaskSnapshot {
    pub task_id: TaskId,
    pub group: Option<String>,
    pub url: String,
    pub path: PathBuf,
    pub expected_bytes: Option<u64>,
    pub received_bytes: u64,
    pub status: TaskStatus,
}
