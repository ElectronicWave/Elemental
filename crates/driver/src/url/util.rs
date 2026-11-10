use reqwest::Url;

use crate::error::{Error, Result};

pub(crate) fn require_http_scheme(url: &Url, context: impl Into<String>) -> Result<()> {
    if matches!(url.scheme(), "http" | "https") {
        return Ok(());
    }

    Err(Error::UnsupportedUrlScheme {
        context: context.into(),
        url: url.to_string(),
    })
}

pub(crate) fn with_directory_path(mut url: Url) -> Result<Url> {
    if !url.path().ends_with('/') {
        let raw_url = url.to_string();
        url.path_segments_mut()
            .map_err(|()| Error::UrlNotPathBase { url: raw_url })?
            .push("");
    }

    Ok(url)
}

pub(super) fn trim_trailing_slash(raw_url: &str) -> &str {
    raw_url.trim_end_matches('/')
}

pub(super) fn origin_suffix(raw_url: &Url, base_url: &Url) -> Option<String> {
    if !same_origin(raw_url, base_url) {
        return None;
    }

    let raw = raw_url.as_str();
    let base = trim_trailing_slash(base_url.as_str());
    let suffix = raw.strip_prefix(base)?;
    if suffix.is_empty() || suffix.starts_with('/') {
        return Some(suffix.to_owned());
    }

    None
}

pub(super) fn same_origin(raw_url: &Url, base_url: &Url) -> bool {
    raw_url.scheme() == base_url.scheme()
        && raw_url.host_str() == base_url.host_str()
        && raw_url.port_or_known_default() == base_url.port_or_known_default()
}
