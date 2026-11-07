use reqwest::Url;

use elemental_infra::SafeRelativePath;

use crate::error::{Error, Result};

pub(crate) fn artifact_url(base_url: &Url, artifact_path: &SafeRelativePath) -> Result<String> {
    base_url
        .join(artifact_path.as_str())
        .map_err(|source| {
            Error::join_url(
                "join maven artifact url failed",
                base_url.as_str(),
                artifact_path.as_str(),
                source,
            )
        })
        .map(|url| url.to_string())
}

pub(crate) fn ensure_http_scheme(url: &Url, context: impl Into<String>) -> Result<()> {
    if matches!(url.scheme(), "http" | "https") {
        return Ok(());
    }

    Err(Error::UnsupportedUrlScheme {
        context: context.into(),
        url: url.to_string(),
    })
}
