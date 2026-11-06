use std::{fmt, path::PathBuf};

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::downloader::task::{DownloadTask, TaskId};

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
