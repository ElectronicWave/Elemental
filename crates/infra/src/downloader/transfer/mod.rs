use futures::StreamExt;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

use super::core::ElementalDownloader;
use super::header::merge_headers;
use super::materializer::StagedDownload;
use super::report::DownloadPhase;
use super::session::SessionHandler;
use super::task::{DownloadTask, TaskId};
use super::validation::DownloadValidation;
use crate::error::Error;

mod error;

pub(super) use error::AttemptError;
use error::TransferError;

pub(super) async fn download_attempt(
    downloader: &ElementalDownloader,
    handler: &SessionHandler,
    task_id: &TaskId,
    task: &DownloadTask,
    token: &CancellationToken,
) -> Result<u64, AttemptError> {
    let staged = match downloader.materializer.create_staging(task).await {
        Ok(staged) => staged,
        Err(error) => {
            return Err(AttemptError::Transfer(TransferError::new(
                DownloadPhase::Prepare,
                error,
            )));
        }
    };

    let staged_path = staged.path.clone();
    let client = downloader.client.clone();
    let materializer = downloader.materializer.clone();
    let materializer_for_execute = materializer.clone();
    let global_bandwidth_limiter = downloader.bandwidth_limiter.clone();
    let task_for_execute = task.clone();
    let mut validator = DownloadValidation::from_task(task).streaming();

    let executer = async move {
        let StagedDownload { path, file } = staged;
        let file = file
            .ok_or_else(|| TransferError::new(DownloadPhase::Prepare, Error::StagedFileMissing))?;
        let headers = merge_headers(handler.headers(), task_for_execute.header_map());
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

            handler.throttle_session(data.len()).await;
            global_bandwidth_limiter.throttle(data.len()).await;
            validator.update(&data);
            received_bytes = next_received_bytes;
            handler.record_received(task_id, data.len());
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
