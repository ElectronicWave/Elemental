use std::{
    fmt,
    future::Future,
    sync::{
        Arc, MutexGuard as StdMutexGuard,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};
use tokio::sync::{Mutex as AsyncMutex, broadcast, mpsc, watch};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use reqwest::header::HeaderMap;

use crate::downloader::session::DownloadSessionOptions;
use crate::downloader::{
    event::DownloadEvent,
    limits::BandwidthLimiter,
    policy::{FailurePolicy, RetryPolicy},
    report::DownloadProgress,
    state::SessionState,
    task::SessionId,
    worker::QueuedDownloadTask,
};

mod lifecycle;
mod reporting;
mod start;
mod submission;
mod tasks;

use lifecycle::SessionLifecycle;

pub(crate) struct SessionHandler {
    session_id: SessionId,
    name: Option<String>,
    retry_policy: RetryPolicy,
    failure_policy: FailurePolicy,
    workers: TaskTracker,
    sender: std::sync::Mutex<Option<mpsc::Sender<QueuedDownloadTask>>>,
    state: std::sync::Mutex<SessionState>,
    submission: AsyncMutex<()>,
    headers: HeaderMap,
    bandwidth_limiter: Arc<BandwidthLimiter>,
    events: broadcast::Sender<DownloadEvent>,
    next_task_id: AtomicU64,
    pending_tasks: AtomicUsize,
    idle: tokio::sync::Notify,
    lifecycle: SessionLifecycle,
    progress: watch::Sender<DownloadProgress>,
    created_at: Instant,
    token: CancellationToken,
}

impl fmt::Debug for SessionHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandler")
            .field("name", &self.name)
            .field("retry_policy", &self.retry_policy)
            .field("failure_policy", &self.failure_policy)
            .field("lifecycle", &self.lifecycle)
            .field("pending_tasks", &self.pending_tasks.load(Ordering::Acquire))
            .finish()
    }
}

impl SessionHandler {
    fn new(
        session_id: SessionId,
        name: Option<String>,
        options: DownloadSessionOptions,
        sender: mpsc::Sender<QueuedDownloadTask>,
        headers: HeaderMap,
        bandwidth_limiter: Arc<BandwidthLimiter>,
        events: broadcast::Sender<DownloadEvent>,
    ) -> (Self, watch::Receiver<DownloadProgress>) {
        let (progress, progress_receiver) = watch::channel(DownloadProgress::default());
        let handler = Self {
            session_id,
            name,
            retry_policy: options.retry_policy,
            failure_policy: options.failure_policy,
            workers: TaskTracker::new(),
            sender: std::sync::Mutex::new(Some(sender)),
            state: std::sync::Mutex::new(SessionState::new(session_id)),
            submission: AsyncMutex::new(()),
            headers,
            bandwidth_limiter,
            events,
            next_task_id: AtomicU64::new(1),
            pending_tasks: AtomicUsize::new(0),
            idle: tokio::sync::Notify::new(),
            lifecycle: SessionLifecycle::new(),
            progress,
            created_at: Instant::now(),
            token: CancellationToken::new(),
        };
        (handler, progress_receiver)
    }

    pub(crate) fn spawn_worker(&self, future: impl Future<Output = ()> + Send + 'static) {
        self.workers.spawn(future);
    }

    pub(crate) fn emit(&self, event: DownloadEvent) {
        let _ = self.events.send(event);
    }

    pub(crate) async fn wait_idle(&self) {
        loop {
            let notified = self.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            if self.pending_tasks.load(Ordering::Acquire) == 0 {
                return;
            }

            notified.await;
        }
    }

    pub(crate) async fn wait_workers(&self) {
        self.workers.wait().await;
    }

    pub(crate) async fn wait_and_require_success(&self) -> crate::error::Result<()> {
        let _submission_guard = self.submission.lock().await;
        self.wait_idle().await;
        self.build_report().success()
    }

    pub(crate) fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy
    }

    pub(crate) fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub(crate) fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub(crate) async fn throttle_session(&self, bytes: usize) {
        self.bandwidth_limiter.throttle(bytes).await;
    }

    fn lock_state(&self) -> StdMutexGuard<'_, SessionState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_sender(&self) -> StdMutexGuard<'_, Option<mpsc::Sender<QueuedDownloadTask>>> {
        self.sender
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
