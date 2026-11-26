use std::{fmt, path::PathBuf, sync::Arc};

use futures::StreamExt;
use reqwest::{Response, header::HeaderMap};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};
use tokio_util::sync::CancellationToken;

use super::limits::BandwidthLimiter;
use super::materializer::{Materializer, StagedDownload};
use super::report::{DownloadFailure, DownloadPhase};
use super::task::{DownloadTask, TaskId};
use super::validation::{DownloadValidation, StreamingValidator};
use crate::error::Error;

fn merge_headers(base: &HeaderMap, overrides: &HeaderMap) -> HeaderMap {
    if overrides.is_empty() {
        return base.clone();
    }

    if base.is_empty() {
        return overrides.clone();
    }

    let mut merged = base.clone();
    for name in overrides.keys() {
        merged.remove(name);
    }
    for (name, value) in overrides {
        merged.append(name.clone(), value.clone());
    }
    merged
}

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
            Self::Cancelled => DownloadFailure::for_task(
                task_id.clone(),
                group,
                task.url().to_owned(),
                task.path().to_path_buf(),
                DownloadPhase::Download,
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
        let retryable = self.is_retryable();
        DownloadFailure::for_task(
            task_id.clone(),
            group,
            task.url().to_owned(),
            task.path().to_path_buf(),
            self.phase,
            self.source.to_string(),
        )
        .with_http_status(self.http_status)
        .with_retryable(retryable)
    }
}

fn is_retryable_http_status(status: u16) -> bool {
    matches!(status, 408 | 429) || (500..600).contains(&status)
}

pub(super) async fn download_attempt(
    context: TransferContext<'_>,
    task: &DownloadTask,
    on_received: impl FnMut(usize),
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
    let materializer = Arc::clone(context.materializer);
    let token = context.token;
    let executer = execute_download(&context, staged, task, on_received);

    tokio::select! {
        biased;
        result = executer => {
            match result {
                Ok(bytes) => Ok(bytes),
                Err(error) => {
                    abort_staged(&materializer, staged_path).await;
                    Err(AttemptError::Transfer(error))
                }
            }
        }
        () = token.cancelled() => {
            abort_staged(&materializer, staged_path).await;
            Err(AttemptError::Cancelled)
        }
    }
}

async fn execute_download(
    context: &TransferContext<'_>,
    staged: StagedDownload,
    task: &DownloadTask,
    on_received: impl FnMut(usize),
) -> Result<u64, TransferError> {
    let StagedDownload { path, file } = staged;
    let file =
        file.ok_or_else(|| TransferError::new(DownloadPhase::Prepare, Error::StagedFileMissing))?;
    let response = send_download_request(context.client, context.session_headers, task).await?;
    validate_content_length(&response, task)?;

    let mut validator = DownloadValidation::from_task(task).streaming();
    let received_bytes =
        write_response_stream(response, file, context, task, &mut validator, on_received).await?;
    validator
        .finish(&path)
        .map_err(|error| TransferError::new(DownloadPhase::Verify, error))?;
    context
        .materializer
        .commit(StagedDownload { path, file: None }, task)
        .await
        .map_err(|error| TransferError::new(DownloadPhase::Commit, error))?;
    Ok(received_bytes)
}

async fn send_download_request(
    client: &reqwest::Client,
    session_headers: &HeaderMap,
    task: &DownloadTask,
) -> Result<Response, TransferError> {
    let headers = merge_headers(session_headers, task.header_map());
    let response = client
        .get(task.url())
        .headers(headers)
        .send()
        .await
        .map_err(|error| TransferError::from_reqwest(DownloadPhase::Connect, error))?;
    response
        .error_for_status()
        .map_err(|error| TransferError::from_reqwest(DownloadPhase::HttpStatus, error))
}

fn validate_content_length(response: &Response, task: &DownloadTask) -> Result<(), TransferError> {
    if let Some(expected_size) = task.expected_size()
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
                path: task.path().to_path_buf(),
                expected: expected_size,
                actual: actual_size,
            },
        ));
    }
    Ok(())
}

async fn write_response_stream(
    response: Response,
    file: File,
    context: &TransferContext<'_>,
    task: &DownloadTask,
    validator: &mut StreamingValidator,
    mut on_received: impl FnMut(usize),
) -> Result<u64, TransferError> {
    let mut stream = response.bytes_stream();
    let mut output = BufWriter::with_capacity(128 * 1024, file);
    let mut received_bytes: u64 = 0;

    while let Some(item) = stream.next().await {
        let data =
            item.map_err(|error| TransferError::from_reqwest(DownloadPhase::Download, error))?;
        let chunk_bytes = u64::try_from(data.len()).unwrap_or(u64::MAX);
        let next_received_bytes = received_bytes.saturating_add(chunk_bytes);
        if let Some(expected_size) = task.expected_size()
            && next_received_bytes > expected_size
        {
            return Err(TransferError::new(
                DownloadPhase::Verify,
                Error::SizeMismatch {
                    path: task.path().to_path_buf(),
                    expected: expected_size,
                    actual: next_received_bytes,
                },
            ));
        }

        context.session_limiter.throttle(data.len()).await;
        context.global_limiter.throttle(data.len()).await;
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
    Ok(received_bytes)
}

async fn abort_staged(materializer: &Arc<dyn Materializer>, path: PathBuf) {
    let _ = materializer
        .abort(StagedDownload { path, file: None })
        .await;
}
