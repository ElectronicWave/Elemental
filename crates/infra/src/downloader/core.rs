use anyhow::{Context, Result, bail};
use futures::StreamExt;
use reqwest::{ClientBuilder, header::HeaderMap, retry};
use scc::{HashMap, hash_map::OccupiedEntry};
use std::{
    fmt,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    sync::{Mutex as AsyncMutex, Notify, Semaphore, mpsc},
    task::JoinHandle,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use super::anyhost::ANY_HOST;
use super::plan::DownloadPlanner;
pub use super::report::{SessionExecutionReport, TaskExecutionFailure};
use super::storage::StagedDownload;
pub use super::storage::{DownloadStorage, HardlinkCachedStorage, LocalFsStorage};
pub use super::task::{DownloadPlan, DownloadTask, SessionId};
use super::tracking::build_task_id;
pub use super::tracking::{TaskId, TrackedInfo, TrackedTaskStatus};
use super::validation::StreamingValidator;

#[derive(Debug)]
pub struct ElementalDownloader {
    client: reqwest::Client,
    sessions: HashMap<SessionId, Arc<SessionHandler>>,
    pub tracker: Arc<ElementalTaskTracker>,
    connections: Arc<Semaphore>,
    storage: Arc<dyn DownloadStorage>,
    worker_count: usize,
    queue_capacity: usize,
    next_session_id: AtomicU64,
    me: Weak<Self>,
}

#[derive(Debug, Clone)]
pub struct DownloadSession {
    id: SessionId,
    name: Option<String>,
    downloader: Weak<ElementalDownloader>,
}

pub struct SessionHandler {
    name: Option<String>,
    workers: TaskTracker,
    sender: std::sync::Mutex<Option<mpsc::Sender<QueuedDownloadTask>>>,
    report: std::sync::Mutex<SessionExecutionState>,
    submission: AsyncMutex<()>,
    pending: AtomicUsize,
    idle: Notify,
    closed: AtomicBool,
}

impl fmt::Debug for SessionHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandler")
            .field("name", &self.name)
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
struct SessionExecutionState {
    accepted: usize,
    enqueued: usize,
    downloaded: usize,
    skipped: usize,
    failures: Vec<TaskExecutionFailure>,
    cancelled_task_ids: Vec<TaskId>,
}

#[derive(Debug)]
pub struct ElementalTaskTracker {
    pub sessions: HashMap<SessionId, SessionState>,
    downloader: Weak<ElementalDownloader>,
}

#[derive(Debug)]
pub struct SessionState {
    pub tasks: HashMap<TaskId, TrackedInfo>,
    pub bps: DownloadBytesPerSecond,
    pub token: CancellationToken,
    counter: JoinHandle<()>,
}

#[derive(Debug)]
pub struct ElementalDownloaderConfig {
    pub max_connections: usize,
    pub connect_timeout: Duration,
    pub retry_times: u32,
}

impl Default for ElementalDownloaderConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            connect_timeout: Duration::from_secs(10),
            retry_times: 3,
        }
    }
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
        self.name.as_deref()
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

    pub async fn report(&self) -> Result<SessionExecutionReport> {
        self.downloader()?.session_report(self.id).await
    }

    pub async fn close(&self) -> Result<bool> {
        self.downloader()?.close_session(self.id).await
    }

    pub async fn wait_empty(&self) -> Result<()> {
        self.downloader()?.wait_session_empty(self.id).await
    }

    pub async fn wait(&self) -> Result<()> {
        self.downloader()?.wait_session(self.id).await
    }

    pub async fn wait_result(&self) -> Result<SessionExecutionReport> {
        self.downloader()?.wait_session_result(self.id).await
    }

    pub async fn remove(&self) -> Result<()> {
        self.downloader()?.remove_session(self.id).await
    }
}

impl SessionHandler {
    fn new(name: Option<String>, sender: mpsc::Sender<QueuedDownloadTask>) -> Self {
        Self {
            name,
            workers: TaskTracker::new(),
            sender: std::sync::Mutex::new(Some(sender)),
            report: std::sync::Mutex::new(SessionExecutionState::default()),
            submission: AsyncMutex::new(()),
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
        let mut report = self.report.lock().expect("session report mutex poisoned");
        report.accepted += 1;
        report.enqueued += 1;
    }

    fn mark_downloaded(&self) {
        self.report
            .lock()
            .expect("session report mutex poisoned")
            .downloaded += 1;
    }

    fn mark_skipped(&self) {
        self.report
            .lock()
            .expect("session report mutex poisoned")
            .skipped += 1;
    }

    fn mark_accepted_skip(&self) {
        let mut report = self.report.lock().expect("session report mutex poisoned");
        report.accepted += 1;
        report.skipped += 1;
    }

    fn mark_failed(&self, task_id: TaskId, error: String) {
        self.report
            .lock()
            .expect("session report mutex poisoned")
            .failures
            .push(TaskExecutionFailure { task_id, error });
    }

    fn mark_cancelled(&self, task_id: TaskId) {
        self.report
            .lock()
            .expect("session report mutex poisoned")
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

    fn snapshot_report(&self, session_id: SessionId) -> SessionExecutionReport {
        let state = self.report.lock().expect("session report mutex poisoned");
        SessionExecutionReport {
            session_id,
            session_name: self.name.clone(),
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
    pub fn new(session_id: SessionId, downloader: Arc<ElementalDownloader>) -> Self {
        Self {
            tasks: HashMap::new(),
            bps: DownloadBytesPerSecond::default(),
            token: CancellationToken::new(),
            counter: tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    if let Some(mut state) =
                        downloader.tracker.sessions.get_async(&session_id).await
                    {
                        state.bps.value = state.bps.count;
                        state.bps.count = 0;
                    } else {
                        break;
                    }
                }
            }),
        }
    }
}

#[derive(Debug, Default)]
pub struct DownloadBytesPerSecond {
    pub count: usize,
    pub value: usize,
}

impl ElementalTaskTracker {
    pub fn new(downloader: Weak<ElementalDownloader>) -> Self {
        Self {
            sessions: HashMap::new(),
            downloader,
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
        let downloader = self
            .downloader
            .upgrade()
            .context("unexpected downloader drop")?;

        let popout = self
            .sessions
            .upsert_async(session_id, SessionState::new(session_id, downloader))
            .await;

        if let Some(old) = popout {
            old.token.cancel();
            old.counter.abort();
            let _ = old.counter.await;
        }

        Ok(())
    }

    pub async fn cancel_session(&self, session_id: SessionId) {
        if let Some(token) = self.session_token(session_id).await {
            token.cancel();
        }
    }

    pub async fn remove_session(&self, session_id: SessionId) {
        if let Some(token) = self.session_token(session_id).await {
            token.cancel();
        }
        self.sessions.remove_async(&session_id).await;
    }

    pub async fn has_session(&self, session_id: SessionId) -> bool {
        self.sessions.contains_async(&session_id).await
    }

    async fn session_token(&self, session_id: SessionId) -> Option<CancellationToken> {
        self.sessions
            .read_async(&session_id, |_, state| state.token.clone())
            .await
    }
}

impl ElementalDownloader {
    pub fn new() -> Arc<Self> {
        Self::from_parts(reqwest::Client::new(), Arc::new(LocalFsStorage), 8)
    }

    pub fn with_config_default() -> Result<Arc<Self>> {
        Self::with_config(ElementalDownloaderConfig::default())
    }

    pub fn with_config(config: ElementalDownloaderConfig) -> Result<Arc<Self>> {
        Self::with_storage(config, Arc::new(LocalFsStorage))
    }

    pub fn with_storage(
        config: ElementalDownloaderConfig,
        storage: Arc<dyn DownloadStorage>,
    ) -> Result<Arc<Self>> {
        let worker_count = config.max_connections.max(1);
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
        let client = ClientBuilder::new()
            .retry(retry_policy)
            .connect_timeout(config.connect_timeout)
            .build()?;

        Ok(Self::from_parts(client, storage, worker_count))
    }

    fn from_parts(
        client: reqwest::Client,
        storage: Arc<dyn DownloadStorage>,
        worker_count: usize,
    ) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            client,
            sessions: HashMap::new(),
            tracker: Arc::new(ElementalTaskTracker::new(me.clone())),
            connections: Arc::new(Semaphore::new(worker_count)),
            storage,
            worker_count,
            queue_capacity: build_queue_capacity(worker_count),
            next_session_id: AtomicU64::new(1),
            me: me.clone(),
        })
    }

    pub fn with_hardlink_cache(
        config: ElementalDownloaderConfig,
        cache_root: impl Into<std::path::PathBuf>,
    ) -> Result<Arc<Self>> {
        Self::with_storage(config, Arc::new(HardlinkCachedStorage::new(cache_root)))
    }

    pub async fn create_named_session(&self, name: impl Into<String>) -> Result<DownloadSession> {
        self.create_session(Some(name.into())).await
    }

    pub async fn create_unnamed_session(&self) -> Result<DownloadSession> {
        self.create_session(None).await
    }

    pub async fn create_session(&self, name: Option<String>) -> Result<DownloadSession> {
        let session_id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        self.tracker.create_session(session_id).await?;

        let downloader = self.me.upgrade().context("unexpected downloader drop")?;
        let (sender, receiver) = mpsc::channel(self.queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let handler = Arc::new(SessionHandler::new(name.clone(), sender));

        for _ in 0..self.worker_count {
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

    pub async fn get_session(
        &self,
        session_id: SessionId,
    ) -> Option<OccupiedEntry<'_, SessionId, Arc<SessionHandler>>> {
        self.sessions.get_async(&session_id).await
    }

    pub async fn session_report(&self, session_id: SessionId) -> Result<SessionExecutionReport> {
        let handler = self.session_handler(session_id).await?;
        Ok(handler.snapshot_report(session_id))
    }

    pub async fn get_session_state(
        &self,
        session_id: SessionId,
    ) -> Option<OccupiedEntry<'_, SessionId, SessionState>> {
        self.tracker.sessions.get_async(&session_id).await
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

        if self.storage.resolve(&task).await? {
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

    pub async fn wait_session(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        Ok(())
    }

    pub async fn wait_session_result(
        &self,
        session_id: SessionId,
    ) -> Result<SessionExecutionReport> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        Ok(handler.snapshot_report(session_id))
    }

    pub async fn run_plan(&self, plan: DownloadPlan) -> Result<SessionExecutionReport> {
        let session = self.create_session(plan.session_name.clone()).await?;
        if let Err(error) = session.add_tasks(plan.tasks).await {
            let _ = self.remove_session(session.id()).await;
            return Err(error);
        }
        let report = session.wait_result().await?;
        self.remove_session(session.id()).await?;
        Ok(report)
    }

    pub async fn execute_plans(
        &self,
        plans: Vec<DownloadPlan>,
    ) -> Result<Vec<SessionExecutionReport>> {
        let mut reports = Vec::with_capacity(plans.len());
        for plan in plans {
            reports.push(self.run_plan(plan).await?);
        }
        Ok(reports)
    }

    pub async fn execute_planner<P>(&self, planner: &P) -> Result<Vec<SessionExecutionReport>>
    where
        P: DownloadPlanner + ?Sized,
    {
        let plans = planner.plan()?;
        self.execute_plans(plans).await
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

    match downloader.storage.resolve(&task).await {
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
        permit = downloader.connections.clone().acquire_owned() => permit,
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

    let staged_output = downloader.storage.create_staging(&task).await;
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
    let storage = downloader.storage.clone();
    let storage_for_execute = storage.clone();
    let tracker = downloader.tracker.clone();
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
            validator.update(&data);
            if let Some(mut state) = tracker.sessions.get_async(&session_id_cloned).await {
                if let Some(mut tracked) = state.tasks.get_async(&task_id_cloned).await {
                    tracked.recv += data.len();
                }
                state.bps.count += data.len();
            }

            output.write_all(&data).await?;
        }
        output.flush().await?;
        drop(output);
        validator.finish(&path)?;
        storage_for_execute
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
                    let _ = storage
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
            let _ = storage
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
