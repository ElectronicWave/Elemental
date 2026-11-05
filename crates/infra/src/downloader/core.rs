use futures::StreamExt;
use reqwest::{ClientBuilder, header::HeaderMap};
use scc::HashMap;
use std::{
    collections::HashMap as StdHashMap,
    fmt,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    sync::{Mutex as AsyncMutex, Notify, mpsc, watch},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use super::control::{BandwidthLimiter, ConcurrencyController};
use super::materializer::StagedDownload;
pub use super::materializer::{HardlinkCachedMaterializer, Materializer, NoCachedMaterializer};
pub use super::policy::{FailurePolicy, RetryPolicy};
pub use super::report::{
    DownloadFailure, DownloadPhase, DownloadProgress, DownloadReport, TaskOutcome, TaskStatus,
};
use super::task::DownloadSessionOptions;
pub use super::task::{ByteRate, DownloadBatch, DownloadRateLimit, DownloadTask, SessionId};
use super::tracking::build_task_id;
pub use super::tracking::{TaskId, TrackedInfo, TrackedTaskStatus};
use super::validation::StreamingValidator;
use crate::error::{Error, Result};

#[derive(Debug)]
pub struct ElementalDownloader {
    client: reqwest::Client,
    sessions: HashMap<SessionId, Arc<SessionHandler>>,
    tracker: Arc<ElementalTaskTracker>,
    concurrency: Arc<ConcurrencyController>,
    bandwidth_limiter: Arc<BandwidthLimiter>,
    materializer: Arc<dyn Materializer>,
    session_parallelism: usize,
    session_queue_capacity: usize,
    next_session_id: AtomicU64,
    me: Weak<Self>,
}

#[derive(Debug)]
pub struct ElementalDownloaderBuilder {
    config: ElementalDownloaderConfig,
    materializer: Arc<dyn Materializer>,
    client: Option<reqwest::Client>,
}

#[derive(Debug, Clone)]
pub struct DownloadSession {
    id: SessionId,
    name: Option<String>,
    downloader: Weak<ElementalDownloader>,
    progress: watch::Receiver<DownloadProgress>,
}

#[derive(Debug)]
pub struct DownloadSessionBuilder<'a> {
    downloader: &'a ElementalDownloader,
    name: String,
    options: DownloadSessionOptions,
}

pub struct SessionHandler {
    name: Option<String>,
    options: DownloadSessionOptions,
    workers: TaskTracker,
    sender: std::sync::Mutex<Option<mpsc::Sender<QueuedDownloadTask>>>,
    state: std::sync::Mutex<SessionProgressState>,
    submission: AsyncMutex<()>,
    bandwidth_limiter: Arc<BandwidthLimiter>,
    pending: AtomicUsize,
    idle: Notify,
    closed: AtomicBool,
    scheduling_stopped: AtomicBool,
    progress: watch::Sender<DownloadProgress>,
    created_at: Instant,
}

impl fmt::Debug for SessionHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandler")
            .field("name", &self.name)
            .field("options", &self.options)
            .field("is_closed", &self.is_closed())
            .field("pending", &self.pending.load(Ordering::Acquire))
            .finish()
    }
}

#[derive(Debug)]
struct QueuedDownloadTask {
    task_id: TaskId,
    task: DownloadTask,
    group: Option<String>,
    headers: HeaderMap,
}

#[derive(Debug)]
struct SessionProgressState {
    accepted: usize,
    enqueued: usize,
    downloaded: usize,
    skipped: usize,
    bytes_downloaded: u64,
    bytes_skipped: u64,
    bytes_received: u64,
    total_bytes: u64,
    total_bytes_known: bool,
    failures: Vec<DownloadFailure>,
    cancelled_task_ids: Vec<TaskId>,
    records: StdHashMap<TaskId, TaskRecord>,
    record_order: Vec<TaskId>,
    bytes_per_second: DownloadBytesPerSecond,
}

impl Default for SessionProgressState {
    fn default() -> Self {
        Self {
            accepted: 0,
            enqueued: 0,
            downloaded: 0,
            skipped: 0,
            bytes_downloaded: 0,
            bytes_skipped: 0,
            bytes_received: 0,
            total_bytes: 0,
            total_bytes_known: true,
            failures: Vec::new(),
            cancelled_task_ids: Vec::new(),
            records: StdHashMap::new(),
            record_order: Vec::new(),
            bytes_per_second: DownloadBytesPerSecond::default(),
        }
    }
}

impl SessionProgressState {
    fn insert_record(&mut self, record: TaskRecord) {
        let task_id = record.task_id.clone();
        match record.expected_size {
            Some(size) => {
                self.total_bytes = self.total_bytes.saturating_add(size);
            }
            None => {
                self.total_bytes_known = false;
            }
        }
        if !self.records.contains_key(&task_id) {
            self.record_order.push(task_id.clone());
        }
        self.records.insert(task_id, record);
    }

    fn finish_record(&mut self, task_id: &TaskId, status: TaskRecordStatus) {
        if let Some(record) = self.records.get_mut(task_id) {
            record.status = status;
        }
    }

    fn progress(&mut self, now: Instant) -> DownloadProgress {
        let bytes_per_second = self.bytes_per_second.value(now);
        let bytes_per_second = u64::try_from(bytes_per_second).unwrap_or(u64::MAX);
        let tasks_completed = self.downloaded.saturating_add(self.skipped);
        let pending = self.accepted.saturating_sub(
            tasks_completed
                .saturating_add(self.failures.len())
                .saturating_add(self.cancelled_task_ids.len()),
        );
        let bytes_total = self.total_bytes_known.then_some(self.total_bytes);
        let completed_bytes = self.bytes_received.saturating_add(self.bytes_skipped);
        let eta = bytes_total.and_then(|total| {
            let remaining = total.saturating_sub(completed_bytes);
            if remaining == 0 {
                Some(Duration::ZERO)
            } else if bytes_per_second == 0 {
                None
            } else {
                Some(Duration::from_secs(remaining / bytes_per_second))
            }
        });

        DownloadProgress {
            tasks_total: self.accepted,
            tasks_completed,
            tasks_failed: self.failures.len(),
            tasks_skipped: self.skipped,
            tasks_cancelled: self.cancelled_task_ids.len(),
            pending,
            bytes_downloaded: self.bytes_received,
            bytes_total,
            bytes_per_second,
            eta,
        }
    }
}

#[derive(Debug, Clone)]
struct TaskRecord {
    task_id: TaskId,
    group: Option<String>,
    url: String,
    path: PathBuf,
    expected_size: Option<u64>,
    status: TaskRecordStatus,
}

#[derive(Debug, Clone)]
enum TaskRecordStatus {
    Queued,
    Downloaded { bytes: u64, duration: Duration },
    Skipped { bytes: u64 },
    Failed(DownloadFailure),
    Cancelled,
}

#[derive(Debug)]
struct ElementalTaskTracker {
    sessions: HashMap<SessionId, SessionState>,
}

#[derive(Debug)]
struct SessionState {
    tasks: HashMap<TaskId, TrackedInfo>,
    token: CancellationToken,
}

#[derive(Debug)]
pub struct ElementalDownloaderConfig {
    pub max_connections: usize,
    pub session_parallelism: usize,
    pub session_queue_capacity: usize,
    pub connect_timeout: Duration,
    pub rate_limit: DownloadRateLimit,
}

impl Default for ElementalDownloaderConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            session_parallelism: 8,
            session_queue_capacity: build_queue_capacity(8),
            connect_timeout: Duration::from_secs(10),
            rate_limit: DownloadRateLimit::Unlimited,
        }
    }
}

impl Default for ElementalDownloaderBuilder {
    fn default() -> Self {
        Self {
            config: ElementalDownloaderConfig::default(),
            materializer: Arc::new(NoCachedMaterializer),
            client: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ValidatedDownloaderConfig {
    max_connections: usize,
    session_parallelism: usize,
    session_queue_capacity: usize,
    connect_timeout: Duration,
    rate_limit: DownloadRateLimit,
}

impl DownloadSession {
    fn downloader(&self) -> Result<Arc<ElementalDownloader>> {
        self.downloader.upgrade().ok_or(Error::DownloaderDropped)
    }

    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub async fn submit(&self, batch: impl Into<DownloadBatch>) -> Result<()> {
        self.downloader()?.submit(self.id, batch.into()).await
    }

    pub fn progress(&self) -> DownloadProgress {
        self.progress.borrow().clone()
    }

    pub fn progress_receiver(&self) -> watch::Receiver<DownloadProgress> {
        self.progress.clone()
    }

    pub async fn wait_idle(&self) -> Result<()> {
        self.downloader()?.wait_session_idle(self.id).await
    }

    pub async fn success(&self) -> Result<()> {
        self.downloader()?.session_success(self.id).await
    }

    pub async fn finish(self) -> Result<DownloadReport> {
        self.downloader()?.finish_session_report(self.id).await
    }

    pub async fn cancel(self) -> Result<DownloadReport> {
        self.downloader()?.cancel_session_report(self.id).await
    }
}

impl<'a> DownloadSessionBuilder<'a> {
    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.options.parallelism = Some(parallelism);
        self
    }

    pub fn retry(mut self, retry_policy: RetryPolicy) -> Self {
        self.options.retry_policy = retry_policy;
        self
    }

    pub fn failure_policy(mut self, failure_policy: FailurePolicy) -> Self {
        self.options.failure_policy = failure_policy;
        self
    }

    pub fn rate_limit(mut self, rate_limit: DownloadRateLimit) -> Self {
        self.options.rate_limit = Some(rate_limit);
        self
    }

    pub async fn open(self) -> Result<DownloadSession> {
        self.downloader
            .create_session(Some(self.name), self.options)
            .await
    }
}

impl ElementalDownloaderBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn config(mut self, config: ElementalDownloaderConfig) -> Self {
        self.config = config;
        self
    }

    pub fn materializer(mut self, materializer: Arc<dyn Materializer>) -> Self {
        self.materializer = materializer;
        self
    }

    pub fn no_cached_materializer(self) -> Self {
        self.materializer(Arc::new(NoCachedMaterializer))
    }

    pub fn hardlink_cached_materializer(self, cache_root: impl Into<std::path::PathBuf>) -> Self {
        self.materializer(Arc::new(HardlinkCachedMaterializer::new(cache_root)))
    }

    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn build(self) -> Result<Arc<ElementalDownloader>> {
        let config = validate_downloader_config(self.config)?;
        let client = match self.client {
            Some(client) => client,
            None => build_downloader_client(&config)?,
        };

        Ok(ElementalDownloader::from_parts(
            client,
            self.materializer,
            config,
        ))
    }
}

impl SessionHandler {
    fn new(
        name: Option<String>,
        options: DownloadSessionOptions,
        sender: mpsc::Sender<QueuedDownloadTask>,
        bandwidth_limiter: Arc<BandwidthLimiter>,
    ) -> (Self, watch::Receiver<DownloadProgress>) {
        let (progress, progress_receiver) = watch::channel(DownloadProgress::default());
        let handler = Self {
            name,
            options,
            workers: TaskTracker::new(),
            sender: std::sync::Mutex::new(Some(sender)),
            state: std::sync::Mutex::new(SessionProgressState::default()),
            submission: AsyncMutex::new(()),
            bandwidth_limiter,
            pending: AtomicUsize::new(0),
            idle: Notify::new(),
            closed: AtomicBool::new(false),
            scheduling_stopped: AtomicBool::new(false),
            progress,
            created_at: Instant::now(),
        };
        (handler, progress_receiver)
    }

    fn update_state(&self, update: impl FnOnce(&mut SessionProgressState)) {
        let progress = {
            let mut state = self.state.lock().expect("session state mutex poisoned");
            update(&mut state);
            state.progress(Instant::now())
        };
        self.progress.send_replace(progress);
    }

    fn clone_sender(&self) -> Option<mpsc::Sender<QueuedDownloadTask>> {
        self.sender
            .lock()
            .expect("session sender mutex poisoned")
            .clone()
    }

    fn start_task(&self) {
        self.pending.fetch_add(1, Ordering::AcqRel);
    }

    fn mark_enqueued(&self, task_id: TaskId, task: &DownloadTask, group: Option<String>) {
        self.update_state(|state| {
            state.accepted += 1;
            state.enqueued += 1;
            state.insert_record(TaskRecord {
                task_id,
                group,
                url: task.url.clone(),
                path: task.path.clone(),
                expected_size: task.expected_size,
                status: TaskRecordStatus::Queued,
            });
        });
    }

    fn mark_accepted_skip(
        &self,
        task_id: TaskId,
        task: &DownloadTask,
        group: Option<String>,
        bytes: u64,
    ) {
        self.update_state(|state| {
            state.accepted += 1;
            state.skipped += 1;
            state.bytes_skipped = state.bytes_skipped.saturating_add(bytes);
            state.insert_record(TaskRecord {
                task_id,
                group,
                url: task.url.clone(),
                path: task.path.clone(),
                expected_size: task.expected_size,
                status: TaskRecordStatus::Skipped { bytes },
            });
        });
    }

    fn mark_downloaded(&self, task_id: &TaskId, bytes: u64, duration: Duration) {
        self.update_state(|state| {
            state.downloaded += 1;
            state.bytes_downloaded = state.bytes_downloaded.saturating_add(bytes);
            state.finish_record(task_id, TaskRecordStatus::Downloaded { bytes, duration });
        });
    }

    fn mark_skipped(&self, task_id: &TaskId, bytes: u64) {
        self.update_state(|state| {
            state.skipped += 1;
            state.bytes_skipped = state.bytes_skipped.saturating_add(bytes);
            state.finish_record(task_id, TaskRecordStatus::Skipped { bytes });
        });
    }

    fn mark_failed(&self, failure: DownloadFailure) {
        self.update_state(|state| {
            let task_id = failure.task_id.clone();
            state.failures.push(failure.clone());
            state.finish_record(&task_id, TaskRecordStatus::Failed(failure));
        });
    }

    fn mark_cancelled(&self, task_id: &TaskId) {
        self.update_state(|state| {
            state.cancelled_task_ids.push(task_id.clone());
            state.finish_record(task_id, TaskRecordStatus::Cancelled);
        });
    }

    fn record_received(&self, bytes: usize) {
        self.update_state(|state| {
            state.bytes_received = state
                .bytes_received
                .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
            state.bytes_per_second.record(bytes, Instant::now());
        });
    }

    fn finish_task(&self) {
        if self.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.idle.notify_waiters();
        }
    }

    async fn wait_idle(&self) {
        loop {
            let notified = self.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            if self.pending.load(Ordering::Acquire) == 0 {
                return;
            }

            notified.await;
        }
    }

    async fn wait_workers(&self) {
        self.workers.wait().await;
    }

    async fn close_and_wait(&self) {
        self.close().await;
        self.wait_idle().await;
        self.wait_workers().await;
    }

    async fn close(&self) -> bool {
        let _submission_guard = self.submission.lock().await;
        let was_closed = self.closed.swap(true, Ordering::AcqRel);
        self.workers.close();
        self.sender
            .lock()
            .expect("session sender mutex poisoned")
            .take();
        !was_closed
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire) || self.workers.is_closed()
    }

    fn is_scheduling_stopped(&self) -> bool {
        self.scheduling_stopped.load(Ordering::Acquire)
    }

    fn stop_scheduling(&self) {
        self.scheduling_stopped.store(true, Ordering::Release);
    }

    fn apply_failure_policy(&self, token: &CancellationToken) {
        match self.options.failure_policy {
            FailurePolicy::Continue => {}
            FailurePolicy::FinishInFlight => self.stop_scheduling(),
            FailurePolicy::CancelAll => {
                self.stop_scheduling();
                token.cancel();
            }
        }
    }

    fn build_report(&self, session_id: SessionId) -> DownloadReport {
        let state = self.state.lock().expect("session state mutex poisoned");
        let outcomes = state
            .record_order
            .iter()
            .filter_map(|task_id| state.records.get(task_id))
            .map(|record| TaskOutcome {
                task_id: record.task_id.clone(),
                group: record.group.clone(),
                url: record.url.clone(),
                path: record.path.clone(),
                status: match &record.status {
                    TaskRecordStatus::Downloaded { bytes, duration } => TaskStatus::Downloaded {
                        bytes: *bytes,
                        duration: *duration,
                    },
                    TaskRecordStatus::Skipped { bytes } => TaskStatus::Skipped { bytes: *bytes },
                    TaskRecordStatus::Failed(failure) => TaskStatus::Failed(failure.clone()),
                    TaskRecordStatus::Cancelled | TaskRecordStatus::Queued => TaskStatus::Cancelled,
                },
            })
            .collect::<Vec<TaskOutcome>>();
        let total_bytes = state.total_bytes_known.then_some(state.total_bytes);

        DownloadReport {
            session_id,
            session_name: self.name.clone(),
            outcomes,
            downloaded_bytes: state.bytes_downloaded,
            skipped_bytes: state.bytes_skipped,
            total_bytes,
            elapsed: self.created_at.elapsed(),
        }
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            token: CancellationToken::new(),
        }
    }
}

#[derive(Debug)]
struct DownloadBytesPerSecond {
    window_started_at: Instant,
    current_window_bytes: usize,
    value: usize,
}

impl Default for DownloadBytesPerSecond {
    fn default() -> Self {
        Self {
            window_started_at: Instant::now(),
            current_window_bytes: 0,
            value: 0,
        }
    }
}

impl DownloadBytesPerSecond {
    fn record(&mut self, bytes: usize, now: Instant) {
        self.roll_window(now);
        self.current_window_bytes = self.current_window_bytes.saturating_add(bytes);
    }

    fn value(&mut self, now: Instant) -> usize {
        self.roll_window(now);
        self.value
    }

    fn roll_window(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.window_started_at);
        if elapsed < Duration::from_secs(1) {
            return;
        }

        let rate = self.current_window_bytes as f64 / elapsed.as_secs_f64();
        self.value = rate as usize;
        self.current_window_bytes = 0;
        self.window_started_at = now;
    }
}

impl ElementalTaskTracker {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub async fn create_task(&self, session_id: SessionId, task_id: TaskId) {
        if let Some(state) = self.sessions.get_async(&session_id).await {
            state
                .tasks
                .upsert_async(task_id, TrackedInfo::waiting())
                .await;
        }
    }

    pub async fn update_task(
        &self,
        session_id: SessionId,
        task_id: &TaskId,
        status: TrackedTaskStatus,
    ) {
        if let Some(state) = self.sessions.get_async(&session_id).await
            && let Some(mut entry) = state.tasks.get_async(task_id).await
        {
            entry.status = status;
        }
    }

    pub async fn remove_task(&self, session_id: SessionId, task_id: &TaskId) {
        if let Some(state) = self.sessions.get_async(&session_id).await {
            state.tasks.remove_async(task_id).await;
        }
    }

    pub async fn create_session(&self, session_id: SessionId) -> Result<()> {
        let popout = self
            .sessions
            .upsert_async(session_id, SessionState::new())
            .await;

        if let Some(old) = popout {
            old.token.cancel();
        }

        Ok(())
    }

    pub async fn remove_session(&self, session_id: SessionId) {
        if let Some(token) = self.session_token(session_id).await {
            token.cancel();
        }

        self.sessions.remove_async(&session_id).await;
    }

    async fn session_token(&self, session_id: SessionId) -> Option<CancellationToken> {
        self.sessions
            .read_async(&session_id, |_, state| state.token.clone())
            .await
    }
}

impl ElementalDownloader {
    pub fn builder() -> ElementalDownloaderBuilder {
        ElementalDownloaderBuilder::new()
    }

    pub fn new() -> Arc<Self> {
        Self::builder()
            .build()
            .expect("default downloader builder must be valid")
    }

    fn from_parts(
        client: reqwest::Client,
        materializer: Arc<dyn Materializer>,
        config: ValidatedDownloaderConfig,
    ) -> Arc<Self> {
        let concurrency = Arc::new(
            ConcurrencyController::new(config.max_connections)
                .expect("validated downloader max_connections must be valid"),
        );
        let bandwidth_limiter = Arc::new(
            BandwidthLimiter::new(config.rate_limit)
                .expect("validated downloader rate limit must be valid"),
        );
        Arc::new_cyclic(|me| Self {
            client,
            sessions: HashMap::new(),
            tracker: Arc::new(ElementalTaskTracker::new()),
            concurrency,
            bandwidth_limiter,
            materializer,
            session_parallelism: config.session_parallelism,
            session_queue_capacity: config.session_queue_capacity,
            next_session_id: AtomicU64::new(1),
            me: me.clone(),
        })
    }

    pub fn session(&self, name: impl Into<String>) -> DownloadSessionBuilder<'_> {
        DownloadSessionBuilder {
            downloader: self,
            name: name.into(),
            options: DownloadSessionOptions::default(),
        }
    }

    async fn create_session(
        &self,
        name: Option<String>,
        options: DownloadSessionOptions,
    ) -> Result<DownloadSession> {
        options.validate()?;
        let session_id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        self.tracker.create_session(session_id).await?;

        let downloader = self.me.upgrade().ok_or(Error::DownloaderDropped)?;
        let (sender, receiver) = mpsc::channel(self.session_queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let parallelism = options.effective_parallelism(self.session_parallelism);
        let session_rate_limit = options.effective_rate_limit();
        let (handler, progress) = SessionHandler::new(
            name.clone(),
            options,
            sender,
            Arc::new(BandwidthLimiter::new(session_rate_limit)?),
        );
        let handler = Arc::new(handler);

        for _ in 0..parallelism {
            let session_id_cloned = session_id;
            let handler_cloned = handler.clone();
            let receiver_cloned = receiver.clone();
            let downloader_cloned = downloader.clone();
            handler.workers.spawn(async move {
                run_session_worker(
                    downloader_cloned,
                    session_id_cloned,
                    handler_cloned,
                    receiver_cloned,
                )
                .await;
            });
        }

        if let Some(old) = self.sessions.upsert_async(session_id, handler).await {
            old.close().await;
        }

        Ok(DownloadSession {
            id: session_id,
            name,
            downloader: self.me.clone(),
            progress,
        })
    }

    async fn enqueue_task(
        &self,
        session_id: SessionId,
        task: DownloadTask,
        group: Option<String>,
    ) -> Result<()> {
        let headers = HeaderMap::default();
        let handler = self.session_handler(session_id).await?;
        let _submission_guard = handler.submission.lock().await;

        if handler.is_closed() {
            return Err(Error::SessionClosed { session_id });
        }

        if handler.is_scheduling_stopped() {
            return Err(Error::SessionNotAcceptingTasks { session_id });
        }

        let task_id = build_task_id(session_id, &task);
        let skipped_bytes = task.expected_size.unwrap_or(0);
        if self.materializer.resolve(&task).await? {
            handler.mark_accepted_skip(task_id, &task, group, skipped_bytes);
            return Ok(());
        }

        let sender = handler.clone_sender().ok_or(Error::SessionSenderClosed)?;
        self.tracker.create_task(session_id, task_id.clone()).await;
        handler.start_task();
        handler.mark_enqueued(task_id.clone(), &task, group.clone());

        let queued = QueuedDownloadTask {
            task_id: task_id.clone(),
            task,
            group,
            headers,
        };

        if let Err(error) = sender.send(queued).await {
            self.tracker.remove_task(session_id, &task_id).await;
            handler.mark_cancelled(&task_id);
            handler.finish_task();
            return Err(Error::EnqueueTask {
                task_id: task_id.to_string(),
                source: Box::new(error),
            });
        }

        Ok(())
    }

    async fn submit(&self, session_id: SessionId, batch: DownloadBatch) -> Result<()> {
        let (group, tasks) = batch.into_parts();
        for task in tasks {
            self.enqueue_task(session_id, task, group.clone()).await?;
        }
        Ok(())
    }

    async fn wait_session_idle(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        handler.wait_idle().await;
        Ok(())
    }

    async fn session_success(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        handler.wait_idle().await;
        handler.build_report(session_id).success()
    }

    async fn finish_session_report(&self, session_id: SessionId) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        self.remove_session(session_id, handler).await
    }

    async fn cancel_session_report(&self, session_id: SessionId) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        if let Some(token) = self.tracker.session_token(session_id).await {
            token.cancel();
        }
        handler.close_and_wait().await;
        self.remove_session(session_id, handler).await
    }

    async fn remove_session(
        &self,
        session_id: SessionId,
        handler: Arc<SessionHandler>,
    ) -> Result<DownloadReport> {
        let report = handler.build_report(session_id);
        self.sessions.remove_async(&session_id).await;
        self.tracker.remove_session(session_id).await;
        Ok(report)
    }

    pub async fn download(&self, task: DownloadTask) -> Result<DownloadReport> {
        let session = self
            .create_session(None, DownloadSessionOptions::default())
            .await?;
        session.submit(task).await?;
        let report = session.finish().await?;
        report.success()?;
        Ok(report)
    }

    pub fn max_connections(&self) -> usize {
        self.concurrency.max_connections()
    }

    pub fn rate_limit(&self) -> DownloadRateLimit {
        self.bandwidth_limiter.rate_limit()
    }

    pub async fn set_max_connections(&self, max_connections: usize) -> Result<()> {
        self.concurrency.set_max_connections(max_connections).await
    }

    pub async fn set_rate_limit(&self, rate_limit: DownloadRateLimit) -> Result<()> {
        self.bandwidth_limiter.set_rate_limit(rate_limit).await
    }

    async fn session_handler(&self, session_id: SessionId) -> Result<Arc<SessionHandler>> {
        self.sessions
            .read_async(&session_id, |_, handler| handler.clone())
            .await
            .ok_or(Error::SessionNotFound { session_id })
    }
}

async fn run_session_worker(
    downloader: Arc<ElementalDownloader>,
    session_id: SessionId,
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

        if handler.is_scheduling_stopped() {
            finish_task_cancelled(
                downloader.tracker.as_ref(),
                handler.as_ref(),
                session_id,
                &queued_task.task_id,
            )
            .await;
            continue;
        }

        execute_queued_task(downloader.clone(), session_id, handler.clone(), queued_task).await;
    }
}

async fn execute_queued_task(
    downloader: Arc<ElementalDownloader>,
    session_id: SessionId,
    handler: Arc<SessionHandler>,
    queued_task: QueuedDownloadTask,
) {
    let QueuedDownloadTask {
        task_id,
        task,
        group,
        headers,
    } = queued_task;
    let record_task = task.clone();
    let session_token = match downloader
        .tracker
        .sessions
        .read_async(&session_id, |_, state| state.token.clone())
        .await
    {
        Some(token) => token,
        None => {
            handler.mark_cancelled(&task_id);
            handler.finish_task();
            return;
        }
    };

    match downloader.materializer.resolve(&task).await {
        Ok(true) => {
            downloader.tracker.remove_task(session_id, &task_id).await;
            handler.mark_skipped(&task_id, task.expected_size.unwrap_or(0));
            handler.finish_task();
            return;
        }
        Ok(false) => {}
        Err(error) => {
            finish_task_error(
                downloader.tracker.as_ref(),
                handler.as_ref(),
                session_id,
                &task_id,
                task_failure(
                    &task_id,
                    &task,
                    group.clone(),
                    DownloadPhase::Prepare,
                    None,
                    error.to_string(),
                ),
                &session_token,
            )
            .await;
            return;
        }
    }

    let token = session_token.child_token();

    let permit = tokio::select! {
        biased;
        () = token.cancelled() => {
            finish_task_cancelled(
                downloader.tracker.as_ref(),
                handler.as_ref(),
                session_id,
                &task_id,
            )
            .await;
            return;
        }
        permit = downloader.concurrency.acquire_owned() => permit,
    };

    let permit = match permit {
        Ok(permit) => permit,
        Err(error) => {
            finish_task_error(
                downloader.tracker.as_ref(),
                handler.as_ref(),
                session_id,
                &task_id,
                task_failure(
                    &task_id,
                    &record_task,
                    group.clone(),
                    DownloadPhase::Prepare,
                    None,
                    error.to_string(),
                ),
                &session_token,
            )
            .await;
            return;
        }
    };

    downloader
        .tracker
        .update_task(session_id, &task_id, TrackedTaskStatus::ACTIVE)
        .await;

    let started_at = Instant::now();
    let retry_policy = handler.options.retry_policy;
    let mut attempt = 0usize;

    loop {
        attempt += 1;

        match execute_task_attempt(
            downloader.as_ref(),
            handler.as_ref(),
            &task,
            &headers,
            &token,
        )
        .await
        {
            Ok(bytes) => {
                downloader.tracker.remove_task(session_id, &task_id).await;
                handler.mark_downloaded(&task_id, bytes, started_at.elapsed());
                break;
            }
            Err(TaskAttemptError::Cancelled) => {
                mark_task_cancelled(
                    downloader.tracker.as_ref(),
                    handler.as_ref(),
                    session_id,
                    &task_id,
                )
                .await;
                break;
            }
            Err(TaskAttemptError::Failed(error)) => {
                let retryable = error.is_retryable();
                if retryable && attempt < retry_policy.max_attempts {
                    let delay = retry_policy.backoff(attempt);
                    if wait_for_retry(&token, delay).await {
                        continue;
                    }

                    mark_task_cancelled(
                        downloader.tracker.as_ref(),
                        handler.as_ref(),
                        session_id,
                        &task_id,
                    )
                    .await;
                    break;
                }

                let failure = error.into_failure(&task_id, &record_task, group.clone());
                mark_task_error(
                    downloader.tracker.as_ref(),
                    handler.as_ref(),
                    session_id,
                    &task_id,
                    failure,
                )
                .await;
                handler.apply_failure_policy(&session_token);
                break;
            }
        }
    }

    drop(permit);
    handler.finish_task();
}

enum TaskAttemptError {
    Failed(DownloadStepError),
    Cancelled,
}

async fn execute_task_attempt(
    downloader: &ElementalDownloader,
    handler: &SessionHandler,
    task: &DownloadTask,
    headers: &HeaderMap,
    token: &CancellationToken,
) -> std::result::Result<u64, TaskAttemptError> {
    let staged = match downloader.materializer.create_staging(task).await {
        Ok(staged) => staged,
        Err(error) => {
            return Err(TaskAttemptError::Failed(DownloadStepError::new(
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
    let session_bandwidth_limiter = handler.bandwidth_limiter.clone();
    let progress_handler = handler;
    let task_for_execute = task.clone();
    let headers = headers.clone();
    let mut validator = StreamingValidator::from_task(task);
    let executer = async move {
        let StagedDownload { path, file } = staged;
        let file = file.ok_or_else(|| {
            DownloadStepError::new(DownloadPhase::Prepare, Error::StagedFileMissing)
        })?;
        let response = client
            .get(task_for_execute.url.clone())
            .headers(headers)
            .send()
            .await
            .map_err(|error| DownloadStepError::from_reqwest(DownloadPhase::Connect, error))?;
        let mut stream = response
            .error_for_status()
            .map_err(|error| DownloadStepError::from_reqwest(DownloadPhase::Status, error))?
            .bytes_stream();

        let mut output = BufWriter::with_capacity(128 * 1024, file);
        let mut received_bytes: u64 = 0;
        while let Some(item) = stream.next().await {
            let data = item
                .map_err(|error| DownloadStepError::from_reqwest(DownloadPhase::Download, error))?;
            session_bandwidth_limiter.throttle(data.len()).await;
            global_bandwidth_limiter.throttle(data.len()).await;
            validator.update(&data);
            received_bytes = received_bytes.saturating_add(data.len() as u64);
            progress_handler.record_received(data.len());
            output
                .write_all(&data)
                .await
                .map_err(|error| DownloadStepError::new(DownloadPhase::Download, error.into()))?;
        }
        output
            .flush()
            .await
            .map_err(|error| DownloadStepError::new(DownloadPhase::Download, error.into()))?;
        drop(output);
        validator
            .finish(&path)
            .map_err(|error| DownloadStepError::new(DownloadPhase::Verify, error))?;
        materializer_for_execute
            .commit(StagedDownload { path, file: None }, &task_for_execute)
            .await
            .map_err(|error| DownloadStepError::new(DownloadPhase::Commit, error))?;
        Ok::<u64, DownloadStepError>(received_bytes)
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
                    Err(TaskAttemptError::Failed(error))
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
            Err(TaskAttemptError::Cancelled)
        }
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

fn build_queue_capacity(worker_count: usize) -> usize {
    (worker_count.saturating_mul(32)).max(128)
}

fn validate_downloader_config(
    config: ElementalDownloaderConfig,
) -> Result<ValidatedDownloaderConfig> {
    if config.max_connections == 0 {
        return Err(Error::InvalidMaxConnections);
    }

    if config.session_parallelism == 0 {
        return Err(Error::InvalidSessionParallelism);
    }

    if config.session_queue_capacity == 0 {
        return Err(Error::InvalidSessionQueueCapacity);
    }

    Ok(ValidatedDownloaderConfig {
        max_connections: config.max_connections,
        session_parallelism: config.session_parallelism,
        session_queue_capacity: config.session_queue_capacity,
        connect_timeout: config.connect_timeout,
        rate_limit: config.rate_limit,
    })
}

fn build_downloader_client(config: &ValidatedDownloaderConfig) -> Result<reqwest::Client> {
    ClientBuilder::new()
        .connect_timeout(config.connect_timeout)
        .build()
        .map_err(Error::from)
}

struct DownloadStepError {
    phase: DownloadPhase,
    http_status: Option<u16>,
    source: Error,
}

impl DownloadStepError {
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
            DownloadPhase::Status => self
                .http_status
                .is_some_and(|status| is_retryable_http_status(status)),
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
        DownloadFailure {
            task_id: task_id.clone(),
            group,
            url: task.url.clone(),
            path: task.path.clone(),
            phase: self.phase,
            http_status: self.http_status,
            retryable,
            message: self.source.to_string(),
        }
    }
}

fn is_retryable_http_status(status: u16) -> bool {
    matches!(status, 408 | 429) || (500..600).contains(&status)
}

fn task_failure(
    task_id: &TaskId,
    task: &DownloadTask,
    group: Option<String>,
    phase: DownloadPhase,
    http_status: Option<u16>,
    message: String,
) -> DownloadFailure {
    DownloadFailure {
        task_id: task_id.clone(),
        group,
        url: task.url.clone(),
        path: task.path.clone(),
        phase,
        http_status,
        retryable: false,
        message,
    }
}

async fn mark_task_error(
    tracker: &ElementalTaskTracker,
    handler: &SessionHandler,
    session_id: SessionId,
    task_id: &TaskId,
    failure: DownloadFailure,
) {
    let error_message = failure.summary();
    tracker
        .update_task(session_id, task_id, TrackedTaskStatus::ERR(error_message))
        .await;
    handler.mark_failed(failure);
}

async fn finish_task_error(
    tracker: &ElementalTaskTracker,
    handler: &SessionHandler,
    session_id: SessionId,
    task_id: &TaskId,
    failure: DownloadFailure,
    session_token: &CancellationToken,
) {
    mark_task_error(tracker, handler, session_id, task_id, failure).await;
    handler.apply_failure_policy(session_token);
    handler.finish_task();
}

async fn mark_task_cancelled(
    tracker: &ElementalTaskTracker,
    handler: &SessionHandler,
    session_id: SessionId,
    task_id: &TaskId,
) {
    tracker
        .update_task(session_id, task_id, TrackedTaskStatus::CANCELLED)
        .await;
    handler.mark_cancelled(task_id);
}

async fn finish_task_cancelled(
    tracker: &ElementalTaskTracker,
    handler: &SessionHandler,
    session_id: SessionId,
    task_id: &TaskId,
) {
    mark_task_cancelled(tracker, handler, session_id, task_id).await;
    handler.finish_task();
}
