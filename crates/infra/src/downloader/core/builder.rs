use std::sync::Arc;

use elemental_core::http::SwappableClient;

use super::ElementalDownloader;
use crate::downloader::config::{ElementalDownloaderConfig, ValidatedDownloaderConfig};
use crate::downloader::materializer::{
    HardlinkCacheMaterializer, Materializer, UncachedMaterializer,
};
use crate::error::Result;

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

    pub fn build(self) -> Result<Arc<ElementalDownloader>> {
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
