//! Reusable, config-swappable `reqwest::Client` configuration and distribution.
//!
//! [`SwappableClient`] distributes cloned `reqwest::Client` handles from a
//! shared [`HttpClientConfig`]. Because `reqwest::Client` is internally
//! reference-counted, swapping the configuration does not disrupt in-flight
//! requests: requests that already acquired a client continue on the previous
//! instance until completion, while subsequent requests receive the new client.

mod client;
mod config;
mod request;
#[cfg(test)]
mod tests;

pub use client::SwappableClient;
pub use config::{
    DEFAULT_CONNECT_TIMEOUT, DEFAULT_READ_TIMEOUT, DEFAULT_USER_AGENT, HttpClientConfig,
    HttpClientError, Result,
};
pub use request::{HttpRequestError, RequestBuilderExt};
