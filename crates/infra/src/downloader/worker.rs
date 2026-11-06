use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio_util::sync::CancellationToken;

use super::core::ElementalDownloader;
use super::report::{DownloadFailure, DownloadPhase};
use super::session::SessionHandler;
use super::task::{DownloadTask, TaskId};
use super::transfer::{AttemptError, download_attempt};

pub(super) struct QueuedDownloadTask {
    pub(super) task_id: TaskId,
    pub(super) task: DownloadTask,
    pub(super) group: Option<String>,
}

pub(super) async fn run_session_worker(
    downloader: Arc<ElementalDownloader>,
    handler: Arc<SessionHandler>,
    receiver: Arc<AsyncMutex<mpsc::Receiver<QueuedDownloadTask>>>,
) {
    loop {
        let next = {
            let mut receiver = receiver.lock().await;
            receiver.recv().await
        };

        let Some(queued_task) = next else {
            break;
        };

        if handler.is_draining() || handler.token().is_cancelled() {
            handler.finish_cancelled(&queued_task.task_id);
            continue;
        }

        execute_queued_task(downloader.clone(), handler.clone(), queued_task).await;
    }
}

async fn execute_queued_task(
    downloader: Arc<ElementalDownloader>,
    handler: Arc<SessionHandler>,
    queued_task: QueuedDownloadTask,
) {
    let QueuedDownloadTask {
        task_id,
        task,
        group,
    } = queued_task;
    let record_task = task.clone();
    let token = handler.token().child_token();

    if token.is_cancelled() {
        handler.finish_cancelled(&task_id);
        return;
    }

    let resolved = tokio::select! {
        biased;
        () = token.cancelled() => {
            handler.finish_cancelled(&task_id);
            return;
        }
        resolved = downloader.materializer.try_reuse(&task) => resolved,
    };

    match resolved {
        Ok(true) => {
            handler.mark_skipped(&task_id, task.expected_size().unwrap_or(0));
            handler.finish_task();
            return;
        }
        Ok(false) => {}
        Err(error) => {
            handler.finish_failed(DownloadFailure::new(
                &task_id,
                &task,
                group,
                DownloadPhase::Prepare,
                None,
                false,
                error.to_string(),
            ));
            return;
        }
    }

    let started_at = Instant::now();
    let retry_policy = handler.retry_policy();
    let mut attempt = 0usize;
    let mut start_emitted = false;

    loop {
        attempt += 1;

        let permit = tokio::select! {
            biased;
            () = token.cancelled() => {
                handler.finish_cancelled(&task_id);
                return;
            }
            permit = downloader.concurrency.acquire_owned() => permit,
        };

        let permit = match permit {
            Ok(permit) => permit,
            Err(error) => {
                handler.finish_failed(DownloadFailure::new(
                    &task_id,
                    &record_task,
                    group,
                    DownloadPhase::Prepare,
                    None,
                    false,
                    error.to_string(),
                ));
                return;
            }
        };

        if token.is_cancelled() || handler.is_draining() {
            drop(permit);
            handler.finish_cancelled(&task_id);
            return;
        }

        if !start_emitted {
            handler.mark_active(&task_id);
            start_emitted = true;
        }
        let result = download_attempt(
            downloader.as_ref(),
            handler.as_ref(),
            &task_id,
            &task,
            &token,
        )
        .await;
        drop(permit);

        match result {
            Ok(bytes) => {
                handler.mark_downloaded(&task_id, bytes, started_at.elapsed());
                break;
            }
            Err(AttemptError::Cancelled) => {
                handler.finish_cancelled(&task_id);
                return;
            }
            Err(error) => {
                let retryable = error.is_retryable();
                if retryable && attempt < retry_policy.max_attempts {
                    let delay = retry_policy.backoff(attempt);
                    handler.mark_retrying(&task_id, attempt, delay, error.to_string());
                    if wait_for_retry(&token, delay).await {
                        continue;
                    }

                    handler.finish_cancelled(&task_id);
                    return;
                }

                let failure = error.into_failure(&task_id, &record_task, group);
                handler.finish_failed(failure);
                return;
            }
        }
    }

    handler.finish_task();
}

async fn wait_for_retry(token: &CancellationToken, delay: Duration) -> bool {
    if delay.is_zero() {
        return !token.is_cancelled();
    }

    tokio::select! {
        biased;
        () = token.cancelled() => false,
        () = tokio::time::sleep(delay) => true,
    }
}
