use std::{fmt, sync::Arc};

use futures::StreamExt;
use reqwest::header::HeaderMap;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

use super::limits::BandwidthLimiter;
use super::materializer::{Materializer, StagedDownload};
use super::report::{DownloadFailure, DownloadPhase};
use super::task::{DownloadTask, TaskId, merge_headers};
use super::validation::DownloadValidation;
use crate::error::Error;

pub(super) struct TransferContext<'a> {
    pub(super) client: &'a reqwest::Client,
    pub(super) materializer: &'a Arc<dyn Materializer>,
    pub(super) global_limiter: &'a BandwidthLimiter,
    pub(super) session_limiter: &'a BandwidthLimiter,
    pub(super) session_headers: &'a HeaderMap,
    pub(super) token: &'a CancellationToken,
}

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
            Self::Cancelled => DownloadFailure::new(
                task_id.clone(),
                group,
                task.url().to_owned(),
                task.path().to_path_buf(),
                DownloadPhase::Download,
                None,
                false,
                "download attempt was cancelled".to_owned(),
            ),
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
    fn new(phase: DownloadPhase, source: Error) -> Self {
        Self {
            phase,
            http_status: None,
            source,
        }
    }

    fn from_reqwest(phase: DownloadPhase, error: reqwest::Error) -> Self {
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

    fn into_failure(
        self,
        task_id: &TaskId,
        task: &DownloadTask,
        group: Option<String>,
    ) -> DownloadFailure {
        DownloadFailure::new(
            task_id.clone(),
            group,
            task.url().to_owned(),
            task.path().to_path_buf(),
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

pub(super) async fn download_attempt(
    context: TransferContext<'_>,
    task: &DownloadTask,
    mut on_received: impl FnMut(usize),
) -> Result<u64, AttemptError> {
    let staged = match context.materializer.create_staging(task).await {
        Ok(staged) => staged,
        Err(error) => {
            return Err(AttemptError::Transfer(TransferError::new(
                DownloadPhase::Prepare,
                error,
            )));
        }
    };

    let staged_path = staged.path.clone();
    let client = context.client.clone();
    let materializer = Arc::clone(context.materializer);
    let materializer_for_execute = Arc::clone(&materializer);
    let global_limiter = context.global_limiter;
    let session_limiter = context.session_limiter;
    let session_headers = context.session_headers;
    let token = context.token;
    let task_for_execute = task.clone();
    let mut validator = DownloadValidation::from_task(task).streaming();

    let executer = async move {
        let StagedDownload { path, file } = staged;
        let file = file
            .ok_or_else(|| TransferError::new(DownloadPhase::Prepare, Error::StagedFileMissing))?;
        let headers = merge_headers(session_headers, task_for_execute.header_map());
        let response = client
            .get(task_for_execute.url())
            .headers(headers)
            .send()
            .await
            .map_err(|error| TransferError::from_reqwest(DownloadPhase::Connect, error))?;
        let response = response
            .error_for_status()
            .map_err(|error| TransferError::from_reqwest(DownloadPhase::HttpStatus, error))?;

        if let Some(expected_size) = task_for_execute.expected_size()
            && let Some(actual_size) = response.content_length()
            && response
                .headers()
                .get(reqwest::header::CONTENT_ENCODING)
                .is_none()
            && actual_size != expected_size
        {
            return Err(TransferError::new(
                DownloadPhase::Verify,
                Error::SizeMismatch {
                    path: task_for_execute.path().to_path_buf(),
                    expected: expected_size,
                    actual: actual_size,
                },
            ));
        }

        let mut stream = response.bytes_stream();
        let mut output = BufWriter::with_capacity(128 * 1024, file);
        let mut received_bytes: u64 = 0;

        while let Some(item) = stream.next().await {
            let data =
                item.map_err(|error| TransferError::from_reqwest(DownloadPhase::Download, error))?;
            let chunk_bytes = u64::try_from(data.len()).unwrap_or(u64::MAX);
            let next_received_bytes = received_bytes.saturating_add(chunk_bytes);
            if let Some(expected_size) = task_for_execute.expected_size()
                && next_received_bytes > expected_size
            {
                return Err(TransferError::new(
                    DownloadPhase::Verify,
                    Error::SizeMismatch {
                        path: task_for_execute.path().to_path_buf(),
                        expected: expected_size,
                        actual: next_received_bytes,
                    },
                ));
            }

            session_limiter.throttle(data.len()).await;
            global_limiter.throttle(data.len()).await;
            validator.update(&data);
            received_bytes = next_received_bytes;
            on_received(data.len());
            output
                .write_all(&data)
                .await
                .map_err(|error| TransferError::new(DownloadPhase::Download, error.into()))?;
        }

        output
            .flush()
            .await
            .map_err(|error| TransferError::new(DownloadPhase::Download, error.into()))?;
        drop(output);
        validator
            .finish(&path)
            .map_err(|error| TransferError::new(DownloadPhase::Verify, error))?;
        materializer_for_execute
            .commit(StagedDownload { path, file: None }, &task_for_execute)
            .await
            .map_err(|error| TransferError::new(DownloadPhase::Commit, error))?;
        Ok::<u64, TransferError>(received_bytes)
    };

    tokio::select! {
        biased;
        result = executer => {
            match result {
                Ok(bytes) => Ok(bytes),
                Err(error) => {
                    let _ = materializer
                        .abort(StagedDownload {
                            path: staged_path.clone(),
                            file: None,
                        })
                        .await;
                    Err(AttemptError::Transfer(error))
                }
            }
        }
        () = token.cancelled() => {
            let _ = materializer
                .abort(StagedDownload {
                    path: staged_path,
                    file: None,
                })
                .await;
            Err(AttemptError::Cancelled)
        }
    }
}
