use std::future::Future;

use serde::de::DeserializeOwned;
use thiserror::Error;

/// Errors returned when executing and decoding HTTP requests via [`RequestBuilderExt`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HttpRequestError {
    #[error("failed to build HTTP request: {source}")]
    Build {
        #[source]
        source: reqwest::Error,
    },

    #[error("HTTP request failed for {url}: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("HTTP {status} response from {url}: {source}")]
    Status {
        url: String,
        status: reqwest::StatusCode,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to read response body from {url}: {source}")]
    ReadBody {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to decode UTF-8 response from {url}: {source}")]
    DecodeUtf8 {
        url: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("failed to decode JSON response from {url}: {source}")]
    DecodeJson {
        url: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Convenience response-decoding helpers on `reqwest::RequestBuilder`.
pub trait RequestBuilderExt {
    fn send_bytes(
        self,
    ) -> impl Future<Output = std::result::Result<Vec<u8>, HttpRequestError>> + Send;
    fn send_text(
        self,
    ) -> impl Future<Output = std::result::Result<String, HttpRequestError>> + Send;
    fn send_json<T: DeserializeOwned>(
        self,
    ) -> impl Future<Output = std::result::Result<T, HttpRequestError>> + Send;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    async fn send_bytes(self) -> std::result::Result<Vec<u8>, HttpRequestError> {
        let (_, bytes) = send_bytes_with_url(self).await?;
        Ok(bytes)
    }

    async fn send_text(self) -> std::result::Result<String, HttpRequestError> {
        let (url, bytes) = send_bytes_with_url(self).await?;
        String::from_utf8(bytes).map_err(|source| HttpRequestError::DecodeUtf8 { url, source })
    }

    async fn send_json<T: DeserializeOwned>(self) -> std::result::Result<T, HttpRequestError> {
        let (url, bytes) = send_bytes_with_url(self).await?;
        serde_json::from_slice(&bytes)
            .map_err(|source| HttpRequestError::DecodeJson { url, source })
    }
}

async fn send_bytes_with_url(
    builder: reqwest::RequestBuilder,
) -> std::result::Result<(String, Vec<u8>), HttpRequestError> {
    let (client, request) = builder.build_split();
    let request = request.map_err(|source| HttpRequestError::Build { source })?;
    let url = request.url().to_string();
    let response = client
        .execute(request)
        .await
        .map_err(|source| HttpRequestError::Request {
            url: url.clone(),
            source,
        })?;
    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(source) => {
            let status = source
                .status()
                .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            return Err(HttpRequestError::Status {
                url,
                status,
                source,
            });
        }
    };
    let bytes = response
        .bytes()
        .await
        .map_err(|source| HttpRequestError::ReadBody {
            url: url.clone(),
            source,
        })?;
    Ok((url, bytes.to_vec()))
}
