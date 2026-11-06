use std::fmt;

use crate::downloader::report::{DownloadFailure, DownloadPhase};
use crate::downloader::task::{DownloadTask, TaskId};
use crate::error::Error;

pub(crate) enum AttemptError {
    Transfer(TransferError),
    Cancelled,
}

impl fmt::Display for AttemptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transfer(error) => fmt::Display::fmt(error, formatter),
            Self::Cancelled => formatter.write_str("download attempt was cancelled"),
        }
    }
}

impl AttemptError {
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            Self::Transfer(error) => error.is_retryable(),
            Self::Cancelled => false,
        }
    }

    pub(crate) fn into_failure(
        self,
        task_id: &TaskId,
        task: &DownloadTask,
        group: Option<String>,
    ) -> DownloadFailure {
        match self {
            Self::Transfer(error) => error.into_failure(task_id, task, group),
            Self::Cancelled => DownloadFailure {
                task_id: task_id.clone(),
                group,
                url: task.url().to_owned(),
                path: task.path().to_path_buf(),
                phase: DownloadPhase::Download,
                http_status: None,
                retryable: false,
                message: "download attempt was cancelled".to_owned(),
            },
        }
    }
}

pub(crate) struct TransferError {
    phase: DownloadPhase,
    http_status: Option<u16>,
    source: Error,
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, formatter)
    }
}

impl TransferError {
    pub(super) fn new(phase: DownloadPhase, source: Error) -> Self {
        Self {
            phase,
            http_status: None,
            source,
        }
    }

    pub(super) fn from_reqwest(phase: DownloadPhase, error: reqwest::Error) -> Self {
        let http_status = error.status().map(|status| status.as_u16());
        Self {
            phase,
            http_status,
            source: Error::Http(error),
        }
    }

    fn is_retryable(&self) -> bool {
        match self.phase {
            DownloadPhase::Connect => match &self.source {
                Error::Http(error) => !error.is_builder() && !error.is_redirect(),
                _ => false,
            },
            DownloadPhase::HttpStatus => self.http_status.is_some_and(is_retryable_http_status),
            DownloadPhase::Download => match &self.source {
                Error::Http(error) => !error.is_builder() && !error.is_decode(),
                _ => false,
            },
            DownloadPhase::Verify => matches!(
                &self.source,
                Error::SizeMismatch { .. } | Error::Sha1Mismatch { .. }
            ),
            DownloadPhase::Prepare | DownloadPhase::Commit => false,
        }
    }

    pub(super) fn into_failure(
        self,
        task_id: &TaskId,
        task: &DownloadTask,
        group: Option<String>,
    ) -> DownloadFailure {
        DownloadFailure::new(
            task_id,
            task,
            group,
            self.phase,
            self.http_status,
            self.is_retryable(),
            self.source.to_string(),
        )
    }
}

fn is_retryable_http_status(status: u16) -> bool {
    matches!(status, 408 | 429) || (500..600).contains(&status)
}
