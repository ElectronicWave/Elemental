use std::{
    fmt,
    future::Future,
    sync::{
        Arc, MutexGuard as StdMutexGuard,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, MutexGuard, broadcast, mpsc, watch};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::downloader::session::DownloadSessionOptions;
use crate::downloader::{
    event::{DownloadEvent, SessionSnapshot},
    limits::BandwidthLimiter,
    policy::{FailurePolicy, RetryPolicy},
    report::{DownloadFailure, DownloadProgress, DownloadReport},
    state::SessionState,
    task::{DownloadTask, SessionId, TaskId},
    worker::QueuedDownloadTask,
};
use crate::error::{Error, Result};

pub(crate) struct SessionHandler {
    session_id: SessionId,
    name: Option<String>,
    options: DownloadSessionOptions,
    workers: TaskTracker,
    sender: std::sync::Mutex<Option<mpsc::Sender<QueuedDownloadTask>>>,
    state: std::sync::Mutex<SessionState>,
    submission: AsyncMutex<()>,
    bandwidth_limiter: Arc<BandwidthLimiter>,
    events: broadcast::Sender<DownloadEvent>,
    next_task_id: AtomicU64,
    pending_tasks: AtomicUsize,
    idle: tokio::sync::Notify,
    closed: AtomicBool,
    scheduling_stopped: AtomicBool,
    progress: watch::Sender<DownloadProgress>,
    created_at: Instant,
    token: CancellationToken,
}

impl fmt::Debug for SessionHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandler")
            .field("name", &self.name)
            .field("options", &self.options)
            .field("is_closed", &self.is_closed())
            .field("pending_tasks", &self.pending_tasks.load(Ordering::Acquire))
            .finish()
    }
}

impl SessionHandler {
    pub(crate) fn new(
        session_id: SessionId,
        name: Option<String>,
        options: DownloadSessionOptions,
        sender: mpsc::Sender<QueuedDownloadTask>,
        bandwidth_limiter: Arc<BandwidthLimiter>,
        events: broadcast::Sender<DownloadEvent>,
    ) -> (Self, watch::Receiver<DownloadProgress>) {
        let (progress, progress_receiver) = watch::channel(DownloadProgress::default());
        let handler = Self {
            session_id,
            name,
            options,
            workers: TaskTracker::new(),
            sender: std::sync::Mutex::new(Some(sender)),
            state: std::sync::Mutex::new(SessionState::new(session_id)),
            submission: AsyncMutex::new(()),
            bandwidth_limiter,
            events,
            next_task_id: AtomicU64::new(1),
            pending_tasks: AtomicUsize::new(0),
            idle: tokio::sync::Notify::new(),
            closed: AtomicBool::new(false),
            scheduling_stopped: AtomicBool::new(false),
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

    pub(crate) fn allocate_task_id(&self) -> TaskId {
        let sequence = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        format!("{}-{}", self.session_id, sequence)
    }

    pub(crate) async fn submission_guard(&self) -> MutexGuard<'_, ()> {
        self.submission.lock().await
    }

    pub(crate) async fn enqueue_task(
        &self,
        task: DownloadTask,
        group: Option<String>,
    ) -> Result<()> {
        let task_id = self.allocate_task_id();
        let sender = self.clone_sender().ok_or(Error::SessionSenderClosed)?;

        self.start_task();
        self.mark_enqueued(task_id.clone(), &task, group.clone());

        let queued = QueuedDownloadTask {
            task_id: task_id.clone(),
            task,
            group,
        };

        if let Err(error) = sender.send(queued).await {
            self.mark_cancelled(&task_id);
            self.finish_task();
            return Err(Error::EnqueueTask {
                task_id,
                source: Box::new(error),
            });
        }

        Ok(())
    }

    fn with_state(
        &self,
        transition: impl FnOnce(&mut SessionState) -> crate::downloader::state::SessionTransition,
    ) {
        let mut state = self.lock_state();
        let update = transition(&mut state);

        self.progress.send_replace(update.progress);
        for event in update.events {
            self.emit(event);
        }
    }

    pub(crate) fn clone_sender(&self) -> Option<mpsc::Sender<QueuedDownloadTask>> {
        self.lock_sender().clone()
    }

    pub(crate) fn start_task(&self) {
        self.pending_tasks.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn mark_enqueued(
        &self,
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
    ) {
        self.with_state(|state| state.enqueue(task_id, task, group, Instant::now()));
    }

    pub(crate) fn mark_downloaded(&self, task_id: &TaskId, bytes: u64, duration: Duration) {
        self.with_state(|state| state.downloaded(task_id, bytes, duration, Instant::now()));
    }

    pub(crate) fn mark_skipped(&self, task_id: &TaskId, bytes: u64) {
        self.with_state(|state| state.skipped(task_id, bytes, Instant::now()));
    }

    pub(crate) fn mark_active(&self, task_id: &TaskId) {
        self.with_state(|state| state.start(task_id, Instant::now()));
    }

    pub(crate) fn mark_failed(&self, failure: DownloadFailure) {
        self.with_state(|state| state.failed(failure, Instant::now()));
    }

    pub(crate) fn mark_cancelled(&self, task_id: &TaskId) {
        self.with_state(|state| state.cancelled(task_id, Instant::now()));
    }

    pub(crate) fn record_received(&self, task_id: &TaskId, bytes: usize) {
        self.with_state(|state| state.receive(task_id, bytes, Instant::now()));
    }

    pub(crate) fn mark_retrying(
        &self,
        task_id: &TaskId,
        failed_attempt: usize,
        delay: Duration,
        reason: String,
    ) {
        self.with_state(|state| {
            state.retrying(task_id, failed_attempt, delay, reason, Instant::now())
        });
    }

    pub(crate) fn finish_task(&self) {
        if self.pending_tasks.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.idle.notify_waiters();
        }
    }

    pub(crate) fn finish_failed(&self, failure: DownloadFailure) {
        self.mark_failed(failure);
        self.apply_failure_policy();
        self.finish_task();
    }

    pub(crate) fn finish_cancelled(&self, task_id: &TaskId) {
        self.mark_cancelled(task_id);
        self.finish_task();
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

    pub(crate) async fn close_and_wait(&self) {
        self.close().await;
        self.wait_idle().await;
        self.wait_workers().await;
    }

    pub(crate) async fn close(&self) {
        let _submission_guard = self.submission.lock().await;
        self.closed.store(true, Ordering::Release);
        self.workers.close();
        self.lock_sender().take();
    }

    pub(crate) fn abort_detached(&self) {
        self.closed.store(true, Ordering::Release);
        self.stop_scheduling();
        self.token.cancel();
        self.workers.close();
        self.lock_sender().take();
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire) || self.workers.is_closed()
    }

    pub(crate) fn is_scheduling_stopped(&self) -> bool {
        self.scheduling_stopped.load(Ordering::Acquire)
    }

    fn stop_scheduling(&self) {
        self.scheduling_stopped.store(true, Ordering::Release);
    }

    pub(crate) fn apply_failure_policy(&self) {
        match self.options.failure_policy {
            FailurePolicy::Continue => {}
            FailurePolicy::FinishInFlight => self.stop_scheduling(),
            FailurePolicy::CancelAll => {
                self.stop_scheduling();
                self.token.cancel();
            }
        }
    }

    pub(crate) fn retry_policy(&self) -> RetryPolicy {
        self.options.retry_policy
    }

    pub(crate) fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub(crate) fn cancel(&self) {
        self.stop_scheduling();
        self.token.cancel();
    }

    pub(crate) async fn throttle_session(&self, bytes: usize) {
        self.bandwidth_limiter.throttle(bytes).await;
    }

    pub(crate) fn snapshot(&self) -> SessionSnapshot {
        self.lock_state().snapshot(self.name.clone())
    }

    pub(crate) fn build_report(&self) -> DownloadReport {
        self.lock_state()
            .report(self.name.clone(), self.created_at.elapsed())
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
