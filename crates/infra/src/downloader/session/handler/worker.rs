use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit, mpsc};
use tokio_util::sync::CancellationToken;

use super::{QueuedDownloadTask, SessionHandler};
use crate::downloader::config::RetryPolicy;
use crate::downloader::report::{DownloadFailure, DownloadPhase};
use crate::downloader::session::SessionRuntime;
use crate::downloader::state::StateCommand;
use crate::downloader::task::{DownloadTask, TaskId};
use crate::downloader::transfer::{AttemptError, TransferContext, download_attempt};

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
    task_id: &'a TaskId,
    task: &'a DownloadTask,
    group: Option<&'a String>,
    token: &'a CancellationToken,
    retry_policy: RetryPolicy,
}

impl SessionHandler {
    pub(in crate::downloader::session) fn spawn_workers(
        self: &Arc<Self>,
        runtime: &Arc<SessionRuntime>,
        receiver: Arc<AsyncMutex<mpsc::Receiver<QueuedDownloadTask>>>,
        parallelism: usize,
    ) {
        for _ in 0..parallelism {
            let worker_runtime = Arc::clone(runtime);
            let worker_handler = Arc::clone(self);
            let worker_receiver = Arc::clone(&receiver);
            self.spawn_worker(async move {
                worker_handler
                    .run_session_worker(worker_runtime, worker_receiver)
                    .await;
            });
        }
    }

    async fn run_session_worker(
        self: Arc<Self>,
        runtime: Arc<SessionRuntime>,
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

            if self.is_draining() || self.token.is_cancelled() {
                self.finish_cancelled(&queued_task.task_id);
                continue;
            }

            self.execute_queued_task(runtime.as_ref(), queued_task)
                .await;
        }
    }

    async fn execute_queued_task(&self, runtime: &SessionRuntime, queued_task: QueuedDownloadTask) {
        let QueuedDownloadTask {
            task_id,
            task,
            group,
        } = queued_task;
        let record_task = task.clone();
        let token = self.token.child_token();

        if token.is_cancelled() {
            self.finish_cancelled(&task_id);
            return;
        }

        match reuse_existing_task(runtime, &task, &token).await {
            ReuseOutcome::Reused => {
                self.apply_state(StateCommand::Skipped {
                    task_id: &task_id,
                    bytes: task.expected_size().unwrap_or(0),
                });
                self.finish_task();
                return;
            }
            ReuseOutcome::Missing => {}
            ReuseOutcome::Cancelled => {
                self.finish_cancelled(&task_id);
                return;
            }
            ReuseOutcome::Failed(message) => {
                self.finish_failed(prepare_failure(task_id, group, &task, message));
                return;
            }
        }

        match self
            .run_attempts(runtime, &task_id, &task, &record_task, group, &token)
            .await
        {
            AttemptOutcome::Downloaded { bytes, elapsed } => {
                self.apply_state(StateCommand::Downloaded {
                    task_id: &task_id,
                    bytes,
                    duration: elapsed,
                });
                self.finish_task();
            }
            AttemptOutcome::Cancelled => {
                self.finish_cancelled(&task_id);
            }
            AttemptOutcome::Failed(failure) => {
                self.finish_failed(failure);
            }
        }
    }

    async fn run_attempts(
        &self,
        runtime: &SessionRuntime,
        task_id: &TaskId,
        task: &DownloadTask,
        record_task: &DownloadTask,
        group: Option<String>,
        token: &CancellationToken,
    ) -> AttemptOutcome {
        let started_at = Instant::now();
        let retry_policy = self.retry_policy;
        let mut attempt = 0usize;
        let mut start_emitted = false;
        let retry_context = RetryContext {
            task_id,
            task: record_task,
            group: group.as_ref(),
            token,
            retry_policy,
        };

        loop {
            attempt += 1;

            let permit = match acquire_permit(runtime, token).await {
                PermitOutcome::Acquired(permit) => permit,
                PermitOutcome::Cancelled => return AttemptOutcome::Cancelled,
                PermitOutcome::Failed(message) => {
                    return AttemptOutcome::Failed(prepare_failure(
                        task_id.clone(),
                        group.clone(),
                        record_task,
                        message,
                    ));
                }
            };

            if token.is_cancelled() || self.is_draining() {
                drop(permit);
                return AttemptOutcome::Cancelled;
            }

            if !start_emitted {
                self.apply_state(StateCommand::Start(task_id));
                start_emitted = true;
            }
            let client = runtime.client.client();
            let transfer_context = self.transfer_context(&client, runtime, token);
            let result = download_attempt(transfer_context, task, |bytes| {
                self.apply_state(StateCommand::Receive { task_id, bytes });
            })
            .await;
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
                    if let Some(outcome) =
                        self.retry_after_error(&retry_context, attempt, error).await
                    {
                        return outcome;
                    }
                }
            }
        }
    }

    async fn retry_after_error(
        &self,
        context: &RetryContext<'_>,
        attempt: usize,
        error: AttemptError,
    ) -> Option<AttemptOutcome> {
        if !error.is_retryable() || attempt >= context.retry_policy.max_attempts {
            let failure = error.into_failure(context.task_id, context.task, context.group.cloned());
            return Some(AttemptOutcome::Failed(failure));
        }

        let delay = context.retry_policy.backoff(attempt);
        self.apply_state(StateCommand::Retrying {
            task_id: context.task_id,
            failed_attempt: attempt,
            delay,
            reason: error.to_string(),
        });
        if wait_for_retry(context.token, delay).await {
            return None;
        }

        Some(AttemptOutcome::Cancelled)
    }

    fn transfer_context<'a>(
        &'a self,
        client: &'a reqwest::Client,
        runtime: &'a SessionRuntime,
        token: &'a CancellationToken,
    ) -> TransferContext<'a> {
        TransferContext {
            client,
            materializer: &runtime.materializer,
            global_limiter: &runtime.bandwidth_limiter,
            session_limiter: &self.bandwidth_limiter,
            session_headers: &self.headers,
            token,
        }
    }
}

fn prepare_failure(
    task_id: TaskId,
    group: Option<String>,
    task: &DownloadTask,
    message: String,
) -> DownloadFailure {
    DownloadFailure::for_task(
        task_id,
        group,
        task.url().to_owned(),
        task.path().to_path_buf(),
        DownloadPhase::Prepare,
        message,
    )
}

async fn reuse_existing_task(
    runtime: &SessionRuntime,
    task: &DownloadTask,
    token: &CancellationToken,
) -> ReuseOutcome {
    let resolved = tokio::select! {
        biased;
        () = token.cancelled() => return ReuseOutcome::Cancelled,
        resolved = runtime.materializer.try_reuse(task) => resolved,
    };

    match resolved {
        Ok(true) => ReuseOutcome::Reused,
        Ok(false) => ReuseOutcome::Missing,
        Err(error) => ReuseOutcome::Failed(error.to_string()),
    }
}

async fn acquire_permit(runtime: &SessionRuntime, token: &CancellationToken) -> PermitOutcome {
    let permit = tokio::select! {
        biased;
        () = token.cancelled() => return PermitOutcome::Cancelled,
        permit = runtime.concurrency.acquire_owned() => permit,
    };

    match permit {
        Ok(permit) => PermitOutcome::Acquired(permit),
        Err(error) => PermitOutcome::Failed(error.to_string()),
    }
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
