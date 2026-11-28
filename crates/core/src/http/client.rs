use std::fmt;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::config::{HttpClientConfig, Result};

struct ClientState {
    config: HttpClientConfig,
    client: reqwest::Client,
    no_redirect_client: reqwest::Client,
}

impl ClientState {
    fn from_config(config: HttpClientConfig) -> Result<Self> {
        let client = config.build()?;
        let no_redirect_client = config.build_no_redirect()?;
        Ok(Self {
            config,
            client,
            no_redirect_client,
        })
    }
}

/// Config-swappable distributor of `reqwest::Client` instances.
///
/// Cloning a `SwappableClient` shares the same underlying state. Calling
/// [`Self::swap`] or [`Self::update`] builds the replacement clients up front
/// and atomically publishes them: in-flight requests retain their previously
/// cloned `reqwest::Client` until completion, while new requests receive the
/// updated client.
#[derive(Clone)]
pub struct SwappableClient {
    inner: Arc<RwLock<ClientState>>,
    headers: HeaderMap,
}

impl Default for SwappableClient {
    fn default() -> Self {
        Self::try_default().expect("build default HTTP client")
    }
}

impl fmt::Debug for SwappableClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.read_state();
        f.debug_struct("SwappableClient")
            .field("config", &state.config)
            .field("scoped_headers_len", &self.headers.len())
            .finish()
    }
}

impl SwappableClient {
    /// Builds a new `SwappableClient` from the provided [`HttpClientConfig`].
    pub fn new(config: HttpClientConfig) -> Result<Self> {
        let state = ClientState::from_config(config)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(state)),
            headers: HeaderMap::new(),
        })
    }

    /// Builds a new `SwappableClient` using [`HttpClientConfig::default`].
    pub fn try_default() -> Result<Self> {
        Self::new(HttpClientConfig::default())
    }

    /// Wraps an already-built `reqwest::Client`.
    ///
    /// Both [`Self::client`] and [`Self::no_redirect_client`] return clones of
    /// `client` until [`Self::swap`] or [`Self::update`] replaces the state.
    pub fn from_client(client: reqwest::Client) -> Self {
        let state = ClientState {
            config: HttpClientConfig::default(),
            no_redirect_client: client.clone(),
            client,
        };
        Self {
            inner: Arc::new(RwLock::new(state)),
            headers: HeaderMap::new(),
        }
    }

    /// Replaces per-handle headers attached to requests created via [`Self::get`],
    /// while continuing to share the underlying hot-swappable client state.
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Inserts a per-handle header attached to requests created via [`Self::get`],
    /// while continuing to share the underlying hot-swappable client state.
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Returns the per-handle headers attached by [`Self::get`].
    pub fn scoped_headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Starts an HTTP request using the active `reqwest::Client` and this handle's scoped headers.
    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl reqwest::IntoUrl,
    ) -> reqwest::RequestBuilder {
        let builder = self.client().request(method, url);
        if self.headers.is_empty() {
            builder
        } else {
            builder.headers(self.headers.clone())
        }
    }

    /// Starts a `GET` request using the active `reqwest::Client` and this handle's scoped headers.
    pub fn get(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.request(reqwest::Method::GET, url)
    }

    /// Returns a clone of the currently active [`HttpClientConfig`].
    pub fn config(&self) -> HttpClientConfig {
        self.read_state().config.clone()
    }

    /// Returns a cheap clone of the currently active `reqwest::Client`.
    pub fn client(&self) -> reqwest::Client {
        self.read_state().client.clone()
    }

    /// Returns a cheap clone of the currently active no-redirect `reqwest::Client`.
    pub fn no_redirect_client(&self) -> reqwest::Client {
        self.read_state().no_redirect_client.clone()
    }

    /// Builds replacement clients from `config` and swaps them in atomically.
    ///
    /// If building either client fails, the previous clients remain active.
    pub fn swap(&self, config: HttpClientConfig) -> Result<()> {
        let next_state = ClientState::from_config(config)?;
        *self.write_state() = next_state;
        Ok(())
    }

    /// Replaces the active `reqwest::Client` directly with an already-built instance.
    pub fn swap_client(&self, client: reqwest::Client) {
        let mut state = self.write_state();
        state.no_redirect_client = client.clone();
        state.client = client;
    }

    /// Clones the active [`HttpClientConfig`], transforms it with `configure`,
    /// and swaps in the rebuilt clients.
    ///
    /// If building fails, the previous configuration and clients remain active.
    pub fn update<F>(&self, configure: F) -> Result<()>
    where
        F: FnOnce(HttpClientConfig) -> HttpClientConfig,
    {
        let next_config = configure(self.config());
        self.swap(next_config)
    }

    fn read_state(&self) -> RwLockReadGuard<'_, ClientState> {
        self.inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_state(&self) -> RwLockWriteGuard<'_, ClientState> {
        self.inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
