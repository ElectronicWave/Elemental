use anyhow::{Context, Result, bail};
use futures::StreamExt;
use reqwest::{ClientBuilder, header::HeaderMap, retry};
use scc::HashMap;
use std::{
    fmt,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    sync::{Mutex as AsyncMutex, Notify, mpsc},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use super::anyhost::ANY_HOST;
use super::control::{BandwidthLimiter, ConcurrencyController};
use super::materializer::StagedDownload;
pub use super::materializer::{HardlinkCachedMaterializer, Materializer, NoCachedMaterializer};
use super::plan::DownloadPlanner;
pub use super::session::{DownloadSessionSnapshot, TaskExecutionFailure};
pub use super::task::{
    ByteRate, DownloadExecutionPolicy, DownloadPlan, DownloadRateLimit, DownloadSessionRequest,
    DownloadTask, SessionId,
};
use super::tracking::build_task_id;
pub use super::tracking::{TaskId, TrackedInfo, TrackedTaskStatus};
use super::validation::StreamingValidator;

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
    request: DownloadSessionRequest,
    downloader: Weak<ElementalDownloader>,
}

pub struct SessionHandler {
    request: DownloadSessionRequest,
    workers: TaskTracker,
    sender: std::sync::Mutex<Option<mpsc::Sender<QueuedDownloadTask>>>,
    state: std::sync::Mutex<SessionProgressState>,
    submission: AsyncMutex<()>,
    bandwidth_limiter: Arc<BandwidthLimiter>,
    pending: AtomicUsize,
    idle: Notify,
    closed: AtomicBool,
}

impl fmt::Debug for SessionHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandler")
            .field("request", &self.request)
            .field("is_closed", &self.is_closed())
            .field("pending", &self.pending.load(Ordering::Acquire))
            .finish()
    }
}

#[derive(Debug)]
struct QueuedDownloadTask {
    task_id: TaskId,
    task: DownloadTask,
    headers: HeaderMap,
}

#[derive(Debug, Default)]
struct SessionProgressState {
    accepted: usize,
    enqueued: usize,
    downloaded: usize,
    skipped: usize,
    failures: Vec<TaskExecutionFailure>,
    cancelled_task_ids: Vec<TaskId>,
}

#[derive(Debug, Clone)]
struct SessionSnapshotData {
    session_name: Option<String>,
    accepted: usize,
    enqueued: usize,
    downloaded: usize,
    skipped: usize,
    failed: usize,
    cancelled: usize,
    pending: usize,
    is_closed: bool,
    failures: Vec<TaskExecutionFailure>,
    cancelled_task_ids: Vec<TaskId>,
}

#[derive(Debug)]
struct ElementalTaskTracker {
    sessions: HashMap<SessionId, SessionState>,
}

#[derive(Debug)]
struct SessionState {
    tasks: HashMap<TaskId, TrackedInfo>,
    bps: DownloadBytesPerSecond,
    token: CancellationToken,
}

#[derive(Debug)]
pub struct ElementalDownloaderConfig {
    pub max_connections: usize,
    pub session_parallelism: usize,
    pub session_queue_capacity: usize,
    pub connect_timeout: Duration,
    pub retry_times: u32,
    pub rate_limit: DownloadRateLimit,
}

impl Default for ElementalDownloaderConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            session_parallelism: 8,
            session_queue_capacity: build_queue_capacity(8),
            connect_timeout: Duration::from_secs(10),
            retry_times: 3,
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
    retry_times: u32,
    rate_limit: DownloadRateLimit,
}

impl DownloadSession {
    fn downloader(&self) -> Result<Arc<ElementalDownloader>> {
        self.downloader
            .upgrade()
            .context("unexpected downloader drop")
    }

    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.request.name.as_deref()
    }

    pub fn request(&self) -> &DownloadSessionRequest {
        &self.request
    }

    pub async fn add_task(&self, task: DownloadTask) -> Result<()> {
        self.downloader()?.add_task(self.id, task).await
    }

    pub async fn add_tasks(&self, tasks: Vec<DownloadTask>) -> Result<()> {
        self.downloader()?.add_tasks(self.id, tasks).await
    }

    pub async fn add_plan(&self, plan: DownloadPlan) -> Result<()> {
        self.downloader()?.add_plan(self.id, plan).await
    }

    pub async fn snapshot(&self) -> Result<DownloadSessionSnapshot> {
        self.downloader()?.session_snapshot(self.id).await
    }

    pub async fn close(&self) -> Result<bool> {
        self.downloader()?.close_session(self.id).await
    }

    pub async fn wait_empty(&self) -> Result<()> {
        self.downloader()?.wait_session_empty(self.id).await
    }

    pub async fn finish(&self) -> Result<()> {
        self.downloader()?.finish_session(self.id).await
    }

    pub async fn finish_snapshot(&self) -> Result<DownloadSessionSnapshot> {
        self.downloader()?.finish_session_snapshot(self.id).await
    }

    pub async fn remove(&self) -> Result<()> {
        self.downloader()?.remove_session(self.id).await
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
        request: DownloadSessionRequest,
        sender: mpsc::Sender<QueuedDownloadTask>,
        bandwidth_limiter: Arc<BandwidthLimiter>,
    ) -> Self {
        Self {
            request,
            workers: TaskTracker::new(),
            sender: std::sync::Mutex::new(Some(sender)),
            state: std::sync::Mutex::new(SessionProgressState::default()),
            submission: AsyncMutex::new(()),
            bandwidth_limiter,
            pending: AtomicUsize::new(0),
            idle: Notify::new(),
            closed: AtomicBool::new(false),
        }
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

    fn mark_enqueued(&self) {
        let mut state = self.state.lock().expect("session state mutex poisoned");
        state.accepted += 1;
        state.enqueued += 1;
    }

    fn mark_downloaded(&self) {
        self.state
            .lock()
            .expect("session state mutex poisoned")
            .downloaded += 1;
    }

    fn mark_skipped(&self) {
        self.state
            .lock()
            .expect("session state mutex poisoned")
            .skipped += 1;
    }

    fn mark_accepted_skip(&self) {
        let mut state = self.state.lock().expect("session state mutex poisoned");
        state.accepted += 1;
        state.skipped += 1;
    }

    fn mark_failed(&self, task_id: TaskId, error: String) {
        self.state
            .lock()
            .expect("session state mutex poisoned")
            .failures
            .push(TaskExecutionFailure { task_id, error });
    }

    fn mark_cancelled(&self, task_id: TaskId) {
        self.state
            .lock()
            .expect("session state mutex poisoned")
            .cancelled_task_ids
            .push(task_id);
    }

    fn finish_task(&self) {
        if self.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.idle.notify_waiters();
        }
    }

    async fn wait_idle(&self) {
        loop {
            if self.pending.load(Ordering::Acquire) == 0 {
                return;
            }

            self.idle.notified().await;
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

    fn snapshot_data(&self) -> SessionSnapshotData {
        let state = self.state.lock().expect("session state mutex poisoned");
        SessionSnapshotData {
            session_name: self.request.name.clone(),
            accepted: state.accepted,
            enqueued: state.enqueued,
            downloaded: state.downloaded,
            skipped: state.skipped,
            failed: state.failures.len(),
            cancelled: state.cancelled_task_ids.len(),
            pending: self.pending.load(Ordering::Acquire),
            is_closed: self.is_closed(),
            failures: state.failures.clone(),
            cancelled_task_ids: state.cancelled_task_ids.clone(),
        }
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            bps: DownloadBytesPerSecond::default(),
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

        self.value = if elapsed < Duration::from_secs(2) {
            self.current_window_bytes
        } else {
            0
        };
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

    pub async fn create_named_session(
        &self,
        name: impl Into<String>,
        execution_policy: DownloadExecutionPolicy,
    ) -> Result<DownloadSession> {
        self.create_session(DownloadSessionRequest::named(name, execution_policy)?)
            .await
    }

    pub async fn create_unnamed_session(
        &self,
        execution_policy: DownloadExecutionPolicy,
    ) -> Result<DownloadSession> {
        self.create_session(DownloadSessionRequest::unnamed(execution_policy)?)
            .await
    }

    pub async fn create_session(&self, request: DownloadSessionRequest) -> Result<DownloadSession> {
        let session_id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        self.tracker.create_session(session_id).await?;

        let downloader = self.me.upgrade().context("unexpected downloader drop")?;
        let (sender, receiver) = mpsc::channel(self.session_queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let parallelism = request.effective_parallelism(self.session_parallelism);
        let session_rate_limit = request.effective_rate_limit();
        let handler = Arc::new(SessionHandler::new(
            request.clone(),
            sender,
            Arc::new(BandwidthLimiter::new(session_rate_limit)?),
        ));

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
            request,
            downloader: self.me.clone(),
        })
    }

    pub async fn close_session(&self, session_id: SessionId) -> Result<bool> {
        let handler = self.session_handler(session_id).await?;
        Ok(handler.close().await)
    }

    pub async fn remove_session(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        self.sessions.remove_async(&session_id).await;
        self.tracker.remove_session(session_id).await;
        Ok(())
    }

    pub async fn has_session(&self, session_id: SessionId) -> bool {
        self.sessions.contains_async(&session_id).await
    }

    pub async fn session_snapshot(&self, session_id: SessionId) -> Result<DownloadSessionSnapshot> {
        let handler = self.session_handler(session_id).await?;
        let snapshot = handler.snapshot_data();
        let bytes_per_second = self
            .tracker
            .sessions
            .get_async(&session_id)
            .await
            .map(|mut state| state.bps.value(Instant::now()))
            .unwrap_or_default();

        Ok(DownloadSessionSnapshot {
            session_id,
            session_name: snapshot.session_name,
            accepted: snapshot.accepted,
            enqueued: snapshot.enqueued,
            downloaded: snapshot.downloaded,
            skipped: snapshot.skipped,
            failed: snapshot.failed,
            cancelled: snapshot.cancelled,
            pending: snapshot.pending,
            bytes_per_second,
            is_closed: snapshot.is_closed,
            failures: snapshot.failures,
            cancelled_task_ids: snapshot.cancelled_task_ids,
        })
    }

    pub async fn add_task(&self, session_id: SessionId, task: DownloadTask) -> Result<()> {
        self.add_task_with_headers(session_id, task, None).await
    }

    pub async fn add_task_with_headers(
        &self,
        session_id: SessionId,
        task: DownloadTask,
        headers: Option<HeaderMap>,
    ) -> Result<()> {
        let headers = headers.unwrap_or_default();
        let handler = self.session_handler(session_id).await?;
        let _submission_guard = handler.submission.lock().await;

        if handler.is_closed() {
            bail!("download session '{}' is closed", session_id);
        }

        if self.materializer.resolve(&task).await? {
            handler.mark_accepted_skip();
            return Ok(());
        }

        let sender = handler
            .clone_sender()
            .context("download session is closed")?;
        let task_id = build_task_id(session_id, &task);
        self.tracker.create_task(session_id, task_id.clone()).await;
        handler.start_task();

        let queued = QueuedDownloadTask {
            task_id: task_id.clone(),
            task,
            headers,
        };

        if let Err(error) = sender.send(queued).await {
            self.tracker.remove_task(session_id, &task_id).await;
            handler.finish_task();
            bail!("failed to enqueue task '{}': {}", task_id, error);
        }

        handler.mark_enqueued();
        Ok(())
    }

    pub async fn add_tasks(&self, session_id: SessionId, tasks: Vec<DownloadTask>) -> Result<()> {
        for task in tasks {
            self.add_task(session_id, task).await?;
        }
        Ok(())
    }

    pub async fn add_plan(&self, session_id: SessionId, plan: DownloadPlan) -> Result<()> {
        self.add_tasks(session_id, plan.tasks).await
    }

    pub async fn wait_session_empty(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        handler.wait_idle().await;
        Ok(())
    }

    pub async fn finish_session(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        Ok(())
    }

    pub async fn finish_session_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<DownloadSessionSnapshot> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        self.session_snapshot(session_id).await
    }

    pub async fn run_plan(&self, plan: DownloadPlan) -> Result<()> {
        let session = self.create_session(plan.session.clone()).await?;
        if let Err(error) = session.add_tasks(plan.tasks).await {
            let _ = self.remove_session(session.id()).await;
            return Err(error);
        }
        let snapshot = session.finish_snapshot().await?;
        let execution_result = ensure_session_succeeded(&snapshot);
        let remove_result = self.remove_session(session.id()).await;

        if let Err(error) = execution_result {
            let _ = remove_result;
            return Err(error);
        }

        remove_result?;
        Ok(())
    }

    pub async fn execute_plans(&self, plans: Vec<DownloadPlan>) -> Result<()> {
        for plan in plans {
            self.run_plan(plan).await?;
        }
        Ok(())
    }

    pub async fn execute_planner<P>(&self, planner: &P) -> Result<()>
    where
        P: DownloadPlanner + ?Sized,
    {
        let plans = planner.plan()?;
        self.execute_plans(plans).await
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
            .context("download session not found")
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
        headers,
    } = queued_task;

    match downloader.materializer.resolve(&task).await {
        Ok(true) => {
            downloader.tracker.remove_task(session_id, &task_id).await;
            handler.mark_skipped();
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
                error.to_string(),
            )
            .await;
            return;
        }
    }

    let token = match downloader
        .tracker
        .sessions
        .read_async(&session_id, |_, state| state.token.child_token())
        .await
    {
        Some(token) => token,
        None => {
            handler.mark_cancelled(task_id.clone());
            handler.finish_task();
            return;
        }
    };

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
                error.to_string(),
            )
            .await;
            return;
        }
    };

    downloader
        .tracker
        .update_task(session_id, &task_id, TrackedTaskStatus::ACTIVE)
        .await;

    let staged_output = downloader.materializer.create_staging(&task).await;
    let staged = match staged_output {
        Ok(output) => output,
        Err(error) => {
            mark_task_error(
                downloader.tracker.as_ref(),
                handler.as_ref(),
                session_id,
                &task_id,
                error.to_string(),
            )
            .await;
            drop(permit);
            handler.finish_task();
            return;
        }
    };
    let staged_path = staged.path.clone();
    let client = downloader.client.clone();
    let materializer = downloader.materializer.clone();
    let materializer_for_execute = materializer.clone();
    let tracker = downloader.tracker.clone();
    let global_bandwidth_limiter = downloader.bandwidth_limiter.clone();
    let session_bandwidth_limiter = handler.bandwidth_limiter.clone();
    let session_id_cloned = session_id;
    let task_id_cloned = task_id.clone();
    let mut validator = StreamingValidator::from_task(&task);
    let executer = async move {
        let StagedDownload { path, file } = staged;
        let file = file.context("staged download missing file handle")?;
        let mut stream = client
            .get(task.url.clone())
            .headers(headers)
            .send()
            .await?
            .error_for_status()?
            .bytes_stream();

        let mut output = BufWriter::with_capacity(128 * 1024, file);
        while let Some(item) = stream.next().await {
            let data = item?;
            session_bandwidth_limiter.throttle(data.len()).await;
            global_bandwidth_limiter.throttle(data.len()).await;
            validator.update(&data);
            if let Some(mut state) = tracker.sessions.get_async(&session_id_cloned).await {
                if let Some(mut tracked) = state.tasks.get_async(&task_id_cloned).await {
                    tracked.recv += data.len();
                }
                state.bps.record(data.len(), Instant::now());
            }

            output.write_all(&data).await?;
        }
        output.flush().await?;
        drop(output);
        validator.finish(&path)?;
        materializer_for_execute
            .commit(StagedDownload { path, file: None }, &task)
            .await?;
        anyhow::Ok(())
    };

    tokio::select! {
        biased;
        result = executer => {
            match result {
                Ok(_) => {
                    downloader.tracker.remove_task(session_id, &task_id).await;
                    handler.mark_downloaded();
                }
                Err(error) => {
                    let _ = materializer
                        .abort(StagedDownload {
                            path: staged_path.clone(),
                            file: None,
                        })
                        .await;
                    mark_task_error(
                        downloader.tracker.as_ref(),
                        handler.as_ref(),
                        session_id,
                        &task_id,
                        error.to_string(),
                    )
                    .await;
                }
            }
        }
        () = token.cancelled() => {
            mark_task_cancelled(
                downloader.tracker.as_ref(),
                handler.as_ref(),
                session_id,
                &task_id,
            )
            .await;
            let _ = materializer
                .abort(StagedDownload {
                    path: staged_path,
                    file: None,
                })
                .await;
        }
    }

    drop(permit);
    handler.finish_task();
}

fn build_queue_capacity(worker_count: usize) -> usize {
    (worker_count.saturating_mul(32)).max(128)
}

fn ensure_session_succeeded(snapshot: &DownloadSessionSnapshot) -> Result<()> {
    let session_label = snapshot
        .session_name
        .as_deref()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| snapshot.session_id.to_string());

    if !snapshot.failures.is_empty() {
        let failures = snapshot
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.task_id, failure.error))
            .collect::<Vec<String>>()
            .join("\n");
        bail!("download session '{session_label}' failed:\n{failures}");
    }

    if !snapshot.cancelled_task_ids.is_empty() {
        let cancelled = snapshot
            .cancelled_task_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join("\n");
        bail!("download session '{session_label}' was cancelled:\n{cancelled}");
    }

    Ok(())
}

fn validate_downloader_config(
    config: ElementalDownloaderConfig,
) -> Result<ValidatedDownloaderConfig> {
    if config.max_connections == 0 {
        bail!("downloader max_connections must be greater than zero");
    }

    if config.session_parallelism == 0 {
        bail!("downloader session_parallelism must be greater than zero");
    }

    if config.session_queue_capacity == 0 {
        bail!("downloader session_queue_capacity must be greater than zero");
    }

    Ok(ValidatedDownloaderConfig {
        max_connections: config.max_connections,
        session_parallelism: config.session_parallelism,
        session_queue_capacity: config.session_queue_capacity,
        connect_timeout: config.connect_timeout,
        retry_times: config.retry_times,
        rate_limit: config.rate_limit,
    })
}

fn build_downloader_client(config: &ValidatedDownloaderConfig) -> Result<reqwest::Client> {
    let retry_policy = retry::for_host(ANY_HOST)
        .max_retries_per_request(config.retry_times)
        .classify_fn(|req_rep| {
            if req_rep.error().is_some()
                || matches!(req_rep.status(), Some(status) if status.is_server_error())
            {
                req_rep.retryable()
            } else {
                req_rep.success()
            }
        });

    ClientBuilder::new()
        .retry(retry_policy)
        .connect_timeout(config.connect_timeout)
        .build()
        .map_err(Into::into)
}

async fn mark_task_error(
    tracker: &ElementalTaskTracker,
    handler: &SessionHandler,
    session_id: SessionId,
    task_id: &TaskId,
    error_message: String,
) {
    tracker
        .update_task(
            session_id,
            task_id,
            TrackedTaskStatus::ERR(error_message.clone()),
        )
        .await;
    handler.mark_failed(task_id.clone(), error_message);
}

async fn finish_task_error(
    tracker: &ElementalTaskTracker,
    handler: &SessionHandler,
    session_id: SessionId,
    task_id: &TaskId,
    error_message: String,
) {
    mark_task_error(tracker, handler, session_id, task_id, error_message).await;
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
    handler.mark_cancelled(task_id.clone());
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
