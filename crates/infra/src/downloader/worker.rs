use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit, mpsc};
use tokio_util::sync::CancellationToken;

use super::core::ElementalDownloader;
use super::policy::RetryPolicy;
use super::report::{DownloadFailure, DownloadPhase};
use super::session::SessionHandler;
use super::task::{DownloadTask, TaskId};
use super::transfer::{AttemptError, download_attempt};

enum ReuseOutcome {
    Reused,
    Missing,
    Cancelled,
    Failed(String),
}

enum AttemptOutcome {
    Downloaded { bytes: u64, elapsed: Duration },
    Cancelled,
    Failed(DownloadFailure),
}

enum PermitOutcome {
    Acquired(OwnedSemaphorePermit),
    Cancelled,
    Failed(String),
}

struct RetryContext<'a> {
    handler: &'a SessionHandler,
    task_id: &'a TaskId,
    task: &'a DownloadTask,
    group: Option<&'a String>,
    token: &'a CancellationToken,
    retry_policy: RetryPolicy,
}

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

    match reuse_existing_task(downloader.as_ref(), &task, &token).await {
        ReuseOutcome::Reused => {
            handler.mark_skipped(&task_id, task.expected_size().unwrap_or(0));
            handler.finish_task();
            return;
        }
        ReuseOutcome::Missing => {}
        ReuseOutcome::Cancelled => {
            handler.finish_cancelled(&task_id);
            return;
        }
        ReuseOutcome::Failed(message) => {
            handler.finish_failed(DownloadFailure::new(
                &task_id,
                &task,
                group,
                DownloadPhase::Prepare,
                None,
                false,
                message,
            ));
            return;
        }
    }

    match run_attempts(
        downloader.as_ref(),
        handler.as_ref(),
        &task_id,
        &task,
        &record_task,
        group,
        &token,
    )
    .await
    {
        AttemptOutcome::Downloaded { bytes, elapsed } => {
            handler.mark_downloaded(&task_id, bytes, elapsed);
            handler.finish_task();
        }
        AttemptOutcome::Cancelled => {
            handler.finish_cancelled(&task_id);
        }
        AttemptOutcome::Failed(failure) => {
            handler.finish_failed(failure);
        }
    }
}

async fn reuse_existing_task(
    downloader: &ElementalDownloader,
    task: &DownloadTask,
    token: &CancellationToken,
) -> ReuseOutcome {
    let resolved = tokio::select! {
        biased;
        () = token.cancelled() => return ReuseOutcome::Cancelled,
        resolved = downloader.materializer.try_reuse(task) => resolved,
    };

    match resolved {
        Ok(true) => ReuseOutcome::Reused,
        Ok(false) => ReuseOutcome::Missing,
        Err(error) => ReuseOutcome::Failed(error.to_string()),
    }
}

async fn run_attempts(
    downloader: &ElementalDownloader,
    handler: &SessionHandler,
    task_id: &TaskId,
    task: &DownloadTask,
    record_task: &DownloadTask,
    group: Option<String>,
    token: &CancellationToken,
) -> AttemptOutcome {
    let started_at = Instant::now();
    let retry_policy = handler.retry_policy();
    let mut attempt = 0usize;
    let mut start_emitted = false;
    let retry_context = RetryContext {
        handler,
        task_id,
        task: record_task,
        group: group.as_ref(),
        token,
        retry_policy,
    };

    loop {
        attempt += 1;

        let permit = match acquire_permit(downloader, token).await {
            PermitOutcome::Acquired(permit) => permit,
            PermitOutcome::Cancelled => return AttemptOutcome::Cancelled,
            PermitOutcome::Failed(message) => {
                return AttemptOutcome::Failed(DownloadFailure::new(
                    task_id,
                    record_task,
                    group.clone(),
                    DownloadPhase::Prepare,
                    None,
                    false,
                    message,
                ));
            }
        };

        if token.is_cancelled() || handler.is_draining() {
            drop(permit);
            return AttemptOutcome::Cancelled;
        }

        if !start_emitted {
            handler.mark_active(task_id);
            start_emitted = true;
        }
        let result = download_attempt(downloader, handler, task_id, task, token).await;
        drop(permit);

        match result {
            Ok(bytes) => {
                return AttemptOutcome::Downloaded {
                    bytes,
                    elapsed: started_at.elapsed(),
                };
            }
            Err(AttemptError::Cancelled) => return AttemptOutcome::Cancelled,
            Err(error) => {
                if let Some(outcome) = retry_after_error(&retry_context, attempt, error).await {
                    return outcome;
                }
            }
        }
    }
}

async fn acquire_permit(
    downloader: &ElementalDownloader,
    token: &CancellationToken,
) -> PermitOutcome {
    let permit = tokio::select! {
        biased;
        () = token.cancelled() => return PermitOutcome::Cancelled,
        permit = downloader.concurrency.acquire_owned() => permit,
    };

    match permit {
        Ok(permit) => PermitOutcome::Acquired(permit),
        Err(error) => PermitOutcome::Failed(error.to_string()),
    }
}

async fn retry_after_error(
    context: &RetryContext<'_>,
    attempt: usize,
    error: AttemptError,
) -> Option<AttemptOutcome> {
    if !error.is_retryable() || attempt >= context.retry_policy.max_attempts {
        let failure = error.into_failure(context.task_id, context.task, context.group.cloned());
        return Some(AttemptOutcome::Failed(failure));
    }

    let delay = context.retry_policy.backoff(attempt);
    context
        .handler
        .mark_retrying(context.task_id, attempt, delay, error.to_string());
    if wait_for_retry(context.token, delay).await {
        return None;
    }

    Some(AttemptOutcome::Cancelled)
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
