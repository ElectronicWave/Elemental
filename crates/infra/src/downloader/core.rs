use scc::HashMap;
use std::sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::{Mutex as AsyncMutex, broadcast, mpsc};

use super::batch::DownloadBatch;
use super::config::{
    ElementalDownloaderConfig, ValidatedDownloaderConfig, build_downloader_client,
    validate_downloader_config,
};
use super::event::{DownloadEvent, DownloadEventStream, SessionSnapshot};
use super::limits::{BandwidthLimiter, ConcurrencyController};
use super::materializer::{HardlinkCacheMaterializer, Materializer, UncachedMaterializer};
use super::rate::DownloadRateLimit;
use super::report::DownloadReport;
use super::session::{
    DownloadSession, DownloadSessionBuilder, DownloadSessionOptions, SessionHandler,
};
use super::task::{DownloadTask, SessionId};
use super::worker::run_session_worker;
use crate::error::{Error, Result};

const EVENT_CHANNEL_CAPACITY: usize = 4096;

#[derive(Debug)]
pub struct ElementalDownloader {
    pub(super) client: reqwest::Client,
    sessions: HashMap<SessionId, Arc<SessionHandler>>,
    events: broadcast::Sender<DownloadEvent>,
    pub(super) concurrency: Arc<ConcurrencyController>,
    pub(super) bandwidth_limiter: Arc<BandwidthLimiter>,
    pub(super) materializer: Arc<dyn Materializer>,
    default_session_parallelism: usize,
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

impl Default for ElementalDownloaderBuilder {
    fn default() -> Self {
        Self {
            config: ElementalDownloaderConfig::default(),
            materializer: Arc::new(UncachedMaterializer),
            client: None,
        }
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

    pub fn without_cache(self) -> Self {
        self.materializer(Arc::new(UncachedMaterializer))
    }

    pub fn with_hardlink_cache(self, cache_root: impl Into<std::path::PathBuf>) -> Self {
        self.materializer(Arc::new(HardlinkCacheMaterializer::new(cache_root)))
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
            ConcurrencyController::new(config.max_concurrent_downloads)
                .expect("validated downloader max_concurrent_downloads must be valid"),
        );
        let bandwidth_limiter = Arc::new(
            BandwidthLimiter::new(config.rate_limit)
                .expect("validated downloader rate limit must be valid"),
        );
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Arc::new_cyclic(|me| Self {
            client,
            sessions: HashMap::new(),
            events,
            concurrency,
            bandwidth_limiter,
            materializer,
            default_session_parallelism: config.default_session_parallelism,
            session_queue_capacity: config.session_queue_capacity,
            next_session_id: AtomicU64::new(1),
            me: me.clone(),
        })
    }

    pub fn session(&self, name: impl Into<String>) -> DownloadSessionBuilder<'_> {
        DownloadSessionBuilder::new(self, name.into())
    }

    pub fn subscribe(&self) -> DownloadEventStream {
        DownloadEventStream::new(self.events.subscribe())
    }

    pub(super) async fn create_session(
        &self,
        name: Option<String>,
        options: DownloadSessionOptions,
    ) -> Result<DownloadSession> {
        options.validate()?;
        let session_id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        let downloader = self.me.upgrade().ok_or(Error::DownloaderDropped)?;
        let (sender, receiver) = mpsc::channel(self.session_queue_capacity);
        let receiver = Arc::new(AsyncMutex::new(receiver));
        let parallelism = options.effective_parallelism(self.default_session_parallelism);
        let session_rate_limit = options.effective_rate_limit();
        let (handler, progress) = SessionHandler::new(
            session_id,
            name.clone(),
            options,
            sender,
            Arc::new(BandwidthLimiter::new(session_rate_limit)?),
            self.events.clone(),
        );
        let handler = Arc::new(handler);

        for _ in 0..parallelism {
            let handler_for_worker = handler.clone();
            let receiver_for_worker = receiver.clone();
            let downloader_for_worker = downloader.clone();
            handler.spawn_worker(async move {
                run_session_worker(
                    downloader_for_worker,
                    handler_for_worker,
                    receiver_for_worker,
                )
                .await;
            });
        }

        let handler_for_event = handler.clone();
        if let Some(old) = self.sessions.upsert_async(session_id, handler).await {
            old.close().await;
        }
        handler_for_event.emit(DownloadEvent::SessionOpened {
            session_id,
            session_name: name.clone(),
        });

        Ok(DownloadSession::new(
            session_id,
            name,
            self.me.clone(),
            progress,
            self.events.clone(),
        ))
    }

    pub(super) async fn submit(&self, session_id: SessionId, batch: DownloadBatch) -> Result<()> {
        let (group, tasks) = batch.into_parts();
        if tasks.is_empty() {
            return Ok(());
        }

        for task in &tasks {
            task.validate()?;
        }

        let handler = self.session_handler(session_id).await?;
        let _submission_guard = handler.submission_guard().await;

        if handler.is_closed() {
            return Err(Error::SessionClosed { session_id });
        }

        if handler.is_scheduling_stopped() {
            return Err(Error::SessionNotAcceptingTasks { session_id });
        }

        for task in tasks {
            handler.enqueue_task(task, group.clone()).await?;
        }

        Ok(())
    }

    pub(super) async fn wait_session_idle(&self, session_id: SessionId) -> Result<()> {
        self.session_handler(session_id).await?.wait_idle().await;
        Ok(())
    }

    pub(super) async fn session_snapshot(&self, session_id: SessionId) -> Result<SessionSnapshot> {
        Ok(self.session_handler(session_id).await?.snapshot())
    }

    pub(super) async fn session_success(&self, session_id: SessionId) -> Result<()> {
        let handler = self.session_handler(session_id).await?;
        let _submission_guard = handler.submission_guard().await;
        handler.wait_idle().await;
        handler.build_report().success()
    }

    pub(super) async fn finish_session_report(
        &self,
        session_id: SessionId,
    ) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        handler.close_and_wait().await;
        self.remove_session(session_id).await
    }

    pub(super) async fn cancel_session_report(
        &self,
        session_id: SessionId,
    ) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        handler.cancel();
        handler.close_and_wait().await;
        self.remove_session(session_id).await
    }

    async fn remove_session(&self, session_id: SessionId) -> Result<DownloadReport> {
        let handler = self.session_handler(session_id).await?;
        let report = handler.build_report();
        handler.emit(DownloadEvent::SessionClosed {
            session_id,
            report: report.clone(),
        });
        self.sessions.remove_async(&session_id).await;
        Ok(report)
    }

    pub(super) fn abort_session_detached(&self, session_id: SessionId) {
        let Some((_, handler)) = self.sessions.remove_sync(&session_id) else {
            return;
        };

        handler.abort_detached();
        let events = self.events.clone();
        let report_handler = handler.clone();
        let wait_handler = handler.clone();
        let close = async move {
            wait_handler.wait_workers().await;
            let _ = events.send(DownloadEvent::SessionClosed {
                session_id,
                report: report_handler.build_report(),
            });
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(close);
            }
            Err(_) => {
                handler.emit(DownloadEvent::SessionClosed {
                    session_id,
                    report: handler.build_report(),
                });
            }
        }
    }

    pub async fn download(&self, task: DownloadTask) -> Result<DownloadReport> {
        task.validate()?;
        let session = self
            .create_session(None, DownloadSessionOptions::default())
            .await?;
        session.submit(task).await?;
        let report = session.finish().await?;
        report.success()?;
        Ok(report)
    }

    pub fn max_concurrent_downloads(&self) -> usize {
        self.concurrency.max_concurrent_downloads()
    }

    pub fn rate_limit(&self) -> DownloadRateLimit {
        self.bandwidth_limiter.rate_limit()
    }

    pub async fn set_max_concurrent_downloads(
        &self,
        max_concurrent_downloads: usize,
    ) -> Result<()> {
        self.concurrency
            .set_max_concurrent_downloads(max_concurrent_downloads)
            .await
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
