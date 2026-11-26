use std::sync::Arc;

use elemental_core::http::SwappableClient;
use tokio::sync::broadcast;

use super::config::{DownloadRateLimit, ElementalDownloaderConfig, ValidatedDownloaderConfig};
use super::event::DownloadEventStream;
use super::limits::{BandwidthLimiter, ConcurrencyController};
use super::materializer::{HardlinkCacheMaterializer, Materializer, UncachedMaterializer};
use super::session::{DownloadSessionBuilder, SessionRuntime};
use crate::error::Result;

const EVENT_CHANNEL_CAPACITY: usize = 4096;

#[derive(Debug, Clone)]
pub struct ElementalDownloader {
    runtime: Arc<SessionRuntime>,
}

impl ElementalDownloader {
    pub fn builder() -> ElementalDownloaderBuilder {
        ElementalDownloaderBuilder::new()
    }

    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("default downloader builder must be valid")
    }

    fn from_parts(
        client: SwappableClient,
        materializer: Arc<dyn Materializer>,
        config: ValidatedDownloaderConfig,
    ) -> Self {
        let concurrency = Arc::new(
            ConcurrencyController::new(config.max_concurrent_downloads)
                .expect("validated downloader max_concurrent_downloads must be valid"),
        );
        let bandwidth_limiter = Arc::new(
            BandwidthLimiter::new(config.rate_limit)
                .expect("validated downloader rate limit must be valid"),
        );
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            runtime: Arc::new(SessionRuntime {
                client,
                events,
                concurrency,
                bandwidth_limiter,
                materializer,
                default_session_parallelism: config.default_session_parallelism,
                session_queue_capacity: config.session_queue_capacity,
            }),
        }
    }

    pub fn session(&self, name: impl Into<String>) -> DownloadSessionBuilder {
        DownloadSessionBuilder::new(Arc::clone(&self.runtime), name.into())
    }

    pub fn subscribe(&self) -> DownloadEventStream {
        DownloadEventStream::new(self.runtime.events.subscribe())
    }

    pub fn max_concurrent_downloads(&self) -> usize {
        self.runtime.concurrency.max_concurrent_downloads()
    }

    pub fn rate_limit(&self) -> DownloadRateLimit {
        self.runtime.bandwidth_limiter.rate_limit()
    }

    /// Returns the swappable HTTP client used for download transfers.
    pub fn http_client(&self) -> &SwappableClient {
        &self.runtime.client
    }

    pub async fn set_max_concurrent_downloads(
        &self,
        max_concurrent_downloads: usize,
    ) -> Result<()> {
        self.runtime
            .concurrency
            .set_max_concurrent_downloads(max_concurrent_downloads)
            .await
    }

    pub async fn set_rate_limit(&self, rate_limit: DownloadRateLimit) -> Result<()> {
        self.runtime
            .bandwidth_limiter
            .set_rate_limit(rate_limit)
            .await
    }
}

#[derive(Debug)]
pub struct ElementalDownloaderBuilder {
    config: ElementalDownloaderConfig,
    materializer: Arc<dyn Materializer>,
    client: Option<SwappableClient>,
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

    /// Sets a fixed `reqwest::Client` wrapped in a [`SwappableClient`].
    pub fn client(self, client: reqwest::Client) -> Self {
        self.http_client(SwappableClient::from_client(client))
    }

    /// Sets a shared [`SwappableClient`] used for download transfers.
    pub fn http_client(mut self, client: SwappableClient) -> Self {
        self.client = Some(client);
        self
    }

    pub fn build(self) -> Result<ElementalDownloader> {
        let config = ValidatedDownloaderConfig::try_from(self.config)?;
        let client = match self.client {
            Some(client) => client,
            None => config.http_client()?,
        };

        Ok(ElementalDownloader::from_parts(
            client,
            self.materializer,
            config,
        ))
    }
}
