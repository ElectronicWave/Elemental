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
