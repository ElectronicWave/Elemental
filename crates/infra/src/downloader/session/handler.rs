use std::{
    fmt,
    future::Future,
    sync::{
        Arc, MutexGuard as StdMutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};
use tokio::sync::{Mutex as AsyncMutex, broadcast, mpsc, watch};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use reqwest::header::HeaderMap;

use crate::downloader::session::DownloadSessionOptions;
use crate::downloader::{
    config::{FailurePolicy, RetryPolicy},
    event::DownloadEvent,
    limits::BandwidthLimiter,
    report::DownloadProgress,
    state::SessionState,
    task::{DownloadTask, SessionId, TaskId},
};
use crate::error::Result;

mod lifecycle;
mod reporting;
mod submission;
mod tasks;
mod worker;

use lifecycle::SessionLifecycle;

pub(super) type QueuedTaskReceiver = Arc<AsyncMutex<mpsc::Receiver<QueuedDownloadTask>>>;
pub(super) type CreatedSessionHandler = (
    Arc<SessionHandler>,
    QueuedTaskReceiver,
    watch::Receiver<DownloadProgress>,
);

pub(super) struct QueuedDownloadTask {
    pub(super) task_id: TaskId,
    pub(super) task: DownloadTask,
    pub(super) group: Option<String>,
}

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
    pub(super) fn create(
        session_id: SessionId,
        name: Option<String>,
        options: DownloadSessionOptions,
        queue_capacity: usize,
        events: broadcast::Sender<DownloadEvent>,
    ) -> Result<CreatedSessionHandler> {
        let bandwidth_limiter = Arc::new(BandwidthLimiter::new(options.effective_rate_limit())?);
        let headers = options.headers.clone();
        let (sender, receiver) = mpsc::channel(queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let (handler, progress) = Self::new(
            session_id,
            name,
            options,
            sender,
            headers,
            bandwidth_limiter,
            events,
        );
        Ok((Arc::new(handler), receiver, progress))
    }

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

    pub(crate) fn subscribe_events(&self) -> broadcast::Receiver<DownloadEvent> {
        self.events.subscribe()
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
