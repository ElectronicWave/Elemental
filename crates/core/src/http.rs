//! Reusable, config-swappable `reqwest::Client` configuration and distribution.
//!
//! [`SwappableClient`] distributes cloned `reqwest::Client` handles from a
//! shared [`HttpClientConfig`]. Because `reqwest::Client` is internally
//! reference-counted, swapping the configuration does not disrupt in-flight
//! requests: requests that already acquired a client continue on the previous
//! instance until completion, while subsequent requests receive the new client.

use std::fmt;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
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

/// Result alias for [`crate::http`] operations.
pub type Result<T> = std::result::Result<T, HttpClientError>;

/// Reusable configuration for constructing `reqwest::Client` instances.
///
/// Unlike `reqwest::ClientBuilder`, `HttpClientConfig` is [`Clone`] and can be
/// replayed whenever a [`SwappableClient`] rebuilds its underlying clients.
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
}

impl fmt::Debug for SwappableClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.read_state();
        f.debug_struct("SwappableClient")
            .field("config", &state.config)
            .finish()
    }
}

impl SwappableClient {
    /// Builds a new `SwappableClient` from the provided [`HttpClientConfig`].
    pub fn new(config: HttpClientConfig) -> Result<Self> {
        let state = ClientState::from_config(config)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(state)),
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
        }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use reqwest::header::{HeaderName, HeaderValue};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Notify;

    use super::{HttpClientConfig, SwappableClient};

    async fn read_request_text(stream: &mut tokio::net::TcpStream) -> String {
        let mut buffer = vec![0u8; 4096];
        let mut data = Vec::new();
        loop {
            let read = stream.read(&mut buffer).await.expect("read request");
            if read == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..read]);
            if data.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&data).into_owned()
    }

    #[tokio::test]
    async fn swap_routes_in_flight_requests_to_old_client_and_new_requests_to_new_client() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("listener addr");
        let first_connected = Arc::new(Notify::new());
        let release_first = Arc::new(Notify::new());
        let first_connected_task = Arc::clone(&first_connected);
        let release_first_task = Arc::clone(&release_first);

        let server = tokio::spawn(async move {
            let (mut first_stream, _) = listener.accept().await.expect("accept first");
            let first_req = read_request_text(&mut first_stream).await;
            first_connected_task.notify_one();

            let (mut second_stream, _) = listener.accept().await.expect("accept second");
            let second_req = read_request_text(&mut second_stream).await;
            second_stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nnew")
                .await
                .expect("write second response");

            release_first_task.notified().await;
            first_stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nold")
                .await
                .expect("write first response");

            (first_req, second_req)
        });

        let swappable = SwappableClient::new(
            HttpClientConfig::new()
                .no_proxy(true)
                .user_agent("ElementalTest/1.0"),
        )
        .expect("build initial client");

        let old_client = swappable.client();
        let url = format!("http://{addr}/");
        let first_url = url.clone();
        let first_task = tokio::spawn(async move {
            old_client
                .get(first_url)
                .send()
                .await
                .expect("first request send")
                .text()
                .await
                .expect("first response text")
        });

        first_connected.notified().await;

        swappable
            .update(|cfg| {
                cfg.user_agent("ElementalTest/2.0").default_header(
                    HeaderName::from_static("x-test-gen"),
                    HeaderValue::from_static("two"),
                )
            })
            .expect("swap config");

        let second_body = swappable
            .client()
            .get(&url)
            .send()
            .await
            .expect("second request send")
            .text()
            .await
            .expect("second response text");
        assert_eq!(second_body, "new");

        release_first.notify_one();
        let first_body = first_task.await.expect("join first task");
        assert_eq!(first_body, "old");

        let (first_req, second_req) = server.await.expect("join server");
        let first_lower = first_req.to_ascii_lowercase();
        let second_lower = second_req.to_ascii_lowercase();
        assert!(first_lower.contains("user-agent: elementaltest/1.0"));
        assert!(!first_lower.contains("x-test-gen: two"));
        assert!(second_lower.contains("user-agent: elementaltest/2.0"));
        assert!(second_lower.contains("x-test-gen: two"));
    }

    #[test]
    fn failed_swap_preserves_existing_config_and_client() {
        let swappable = SwappableClient::new(
            HttpClientConfig::new()
                .user_agent("ValidAgent/1.0")
                .connect_timeout(Some(Duration::from_secs(5))),
        )
        .expect("build initial client");

        let result = swappable.swap(HttpClientConfig::new().user_agent("Invalid\r\nHeader"));
        assert!(result.is_err());
        assert_eq!(
            swappable.config().user_agent_value(),
            Some("ValidAgent/1.0")
        );
        assert_eq!(
            swappable.config().connect_timeout_value(),
            Some(Duration::from_secs(5))
        );
    }

    #[tokio::test]
    async fn no_redirect_client_does_not_follow_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind redirect listener");
        let addr = listener.local_addr().expect("redirect listener addr");

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept redirect request");
            let _ = read_request_text(&mut stream).await;
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: /target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("write redirect response");
        });

        let swappable = SwappableClient::new(HttpClientConfig::new().no_proxy(true))
            .expect("build swappable client");
        let response = swappable
            .no_redirect_client()
            .get(format!("http://{addr}/start"))
            .send()
            .await
            .expect("send request");

        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.await.expect("join redirect server");
    }
}
