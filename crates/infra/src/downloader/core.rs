use scc::HashMap;
use std::sync::{Arc, atomic::AtomicU64};
use tokio::sync::broadcast;

use super::config::ValidatedDownloaderConfig;
use super::event::{DownloadEvent, DownloadEventStream};
use super::limits::{BandwidthLimiter, ConcurrencyController};
use super::materializer::Materializer;
use super::rate::DownloadRateLimit;
use super::session::{DownloadSessionBuilder, SessionHandler};
use super::task::SessionId;
use crate::error::Result;

mod builder;
mod sessions;

pub use builder::ElementalDownloaderBuilder;

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
        Arc::new(Self {
            client,
            sessions: HashMap::new(),
            events,
            concurrency,
            bandwidth_limiter,
            materializer,
            default_session_parallelism: config.default_session_parallelism,
            session_queue_capacity: config.session_queue_capacity,
            next_session_id: AtomicU64::new(1),
        })
    }

    pub fn session(self: &Arc<Self>, name: impl Into<String>) -> DownloadSessionBuilder {
        DownloadSessionBuilder::new(Arc::clone(self), name.into())
    }

    pub fn subscribe(&self) -> DownloadEventStream {
        DownloadEventStream::new(self.events.subscribe())
    }

    pub(crate) fn event_receiver(&self) -> broadcast::Receiver<DownloadEvent> {
        self.events.subscribe()
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
}
