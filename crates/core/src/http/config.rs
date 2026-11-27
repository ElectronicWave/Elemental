use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use thiserror::Error;

/// Default `User-Agent` header applied by [`HttpClientConfig::default`].
pub const DEFAULT_USER_AGENT: &str = concat!("Elemental/", env!("CARGO_PKG_VERSION"));

/// Default connection timeout applied by [`HttpClientConfig::default`].
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default socket read timeout applied by [`HttpClientConfig::default`].
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

type ClientCustomizer = Arc<dyn Fn(reqwest::ClientBuilder) -> reqwest::ClientBuilder + Send + Sync>;

/// Errors returned when building or swapping a `reqwest::Client`.
#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error("failed to build HTTP client: {source}")]
    Build {
        #[source]
        source: reqwest::Error,
    },
}

/// Result alias for [`crate::http`] configuration operations.
pub type Result<T> = std::result::Result<T, HttpClientError>;

/// Reusable configuration for constructing `reqwest::Client` instances.
///
/// Unlike `reqwest::ClientBuilder`, `HttpClientConfig` is [`Clone`] and can be
/// replayed whenever a [`super::SwappableClient`] rebuilds its underlying clients.
/// Request-specific headers (such as per-mirror `X-KEY` or per-request bearer
/// tokens) should be attached on the request or source layer rather than placed
/// in [`Self::default_headers`] when a client is shared across origins.
#[derive(Clone)]
pub struct HttpClientConfig {
    user_agent: Option<String>,
    default_headers: HeaderMap,
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    timeout: Option<Duration>,
    proxies: Vec<reqwest::Proxy>,
    no_proxy: bool,
    customizer: Option<ClientCustomizer>,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            user_agent: Some(DEFAULT_USER_AGENT.to_owned()),
            default_headers: HeaderMap::new(),
            connect_timeout: Some(DEFAULT_CONNECT_TIMEOUT),
            read_timeout: Some(DEFAULT_READ_TIMEOUT),
            timeout: None,
            proxies: Vec::new(),
            no_proxy: false,
            customizer: None,
        }
    }
}

impl fmt::Debug for HttpClientConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpClientConfig")
            .field("user_agent", &self.user_agent)
            .field("default_headers_len", &self.default_headers.len())
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("timeout", &self.timeout)
            .field("proxies_len", &self.proxies.len())
            .field("no_proxy", &self.no_proxy)
            .field("has_customizer", &self.customizer.is_some())
            .finish()
    }
}

impl HttpClientConfig {
    /// Creates a new configuration populated with Elemental defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `User-Agent` header value sent by built clients.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Clears the `User-Agent` header so `reqwest` does not send a default one.
    pub fn without_user_agent(mut self) -> Self {
        self.user_agent = None;
        self
    }

    /// Replaces the default headers attached to every request made by built clients.
    pub fn default_headers(mut self, headers: HeaderMap) -> Self {
        self.default_headers = headers;
        self
    }

    /// Inserts a single default header attached to every request made by built clients.
    pub fn default_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Sets the TCP/TLS connect timeout. Pass `None` to disable it.
    pub fn connect_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the per-read inactivity timeout. Pass `None` to disable it.
    pub fn read_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Sets a total request timeout. Pass `None` to disable it.
    ///
    /// Leave this as `None` when sharing the client with streaming downloads so
    /// large transfers are bounded by [`Self::read_timeout`] instead.
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Appends a proxy rule to the client configuration.
    pub fn proxy(mut self, proxy: reqwest::Proxy) -> Self {
        self.proxies.push(proxy);
        self
    }

    /// Replaces all configured proxy rules.
    pub fn proxies(mut self, proxies: impl IntoIterator<Item = reqwest::Proxy>) -> Self {
        self.proxies = proxies.into_iter().collect();
        self
    }

    /// Disables system proxy discovery when `no_proxy` is `true`.
    pub fn no_proxy(mut self, no_proxy: bool) -> Self {
        self.no_proxy = no_proxy;
        self
    }

    /// Registers a reusable hook that customizes the underlying `reqwest::ClientBuilder`.
    ///
    /// Calling this method multiple times composes the hooks in registration order.
    pub fn customize<F>(mut self, customizer: F) -> Self
    where
        F: Fn(reqwest::ClientBuilder) -> reqwest::ClientBuilder + Send + Sync + 'static,
    {
        self.customizer = Some(match self.customizer {
            Some(previous) => Arc::new(move |builder| customizer(previous(builder))),
            None => Arc::new(customizer),
        });
        self
    }

    /// Returns the configured `User-Agent`, if any.
    pub fn user_agent_value(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// Returns the configured default headers.
    pub fn default_headers_map(&self) -> &HeaderMap {
        &self.default_headers
    }

    /// Returns the configured connect timeout.
    pub fn connect_timeout_value(&self) -> Option<Duration> {
        self.connect_timeout
    }

    /// Returns the configured read timeout.
    pub fn read_timeout_value(&self) -> Option<Duration> {
        self.read_timeout
    }

    /// Returns the configured total request timeout.
    pub fn timeout_value(&self) -> Option<Duration> {
        self.timeout
    }

    /// Returns whether system proxy discovery is disabled.
    pub fn is_no_proxy(&self) -> bool {
        self.no_proxy
    }

    /// Produces a fresh `reqwest::ClientBuilder` with this configuration applied.
    pub fn builder(&self) -> reqwest::ClientBuilder {
        let mut builder = reqwest::Client::builder();
        if let Some(user_agent) = &self.user_agent {
            builder = builder.user_agent(user_agent.as_str());
        }
        if !self.default_headers.is_empty() {
            builder = builder.default_headers(self.default_headers.clone());
        }
        if let Some(connect_timeout) = self.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        if let Some(read_timeout) = self.read_timeout {
            builder = builder.read_timeout(read_timeout);
        }
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        if self.no_proxy {
            builder = builder.no_proxy();
        }
        for proxy in &self.proxies {
            builder = builder.proxy(proxy.clone());
        }
        if let Some(customizer) = &self.customizer {
            builder = customizer(builder);
        }
        builder
    }

    /// Builds a standard `reqwest::Client` from this configuration.
    pub fn build(&self) -> Result<reqwest::Client> {
        self.builder()
            .build()
            .map_err(|source| HttpClientError::Build { source })
    }

    /// Builds a `reqwest::Client` from this configuration with redirects disabled.
    ///
    /// Redirect suppression is applied after any customizer hook so security-sensitive
    /// flows such as OAuth token exchange cannot be redirected.
    pub fn build_no_redirect(&self) -> Result<reqwest::Client> {
        self.builder()
            .redirect(Policy::none())
            .build()
            .map_err(|source| HttpClientError::Build { source })
    }
}
