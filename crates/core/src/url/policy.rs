use reqwest::Url;

use super::{
    error::{OriginError, Result},
    util::{require_http_scheme, sanitize_relative_path, trim_trailing_slash, with_directory_path},
};

#[derive(Debug, Clone, Default)]
pub struct OriginPolicy {
    pub(super) overrides: Vec<(String, Url)>,
}

impl OriginPolicy {
    pub fn new(overrides: impl IntoIterator<Item = (impl AsRef<str>, Url)>) -> Result<Self> {
        let mut policy = Self::default();
        for (origin, url) in overrides {
            require_http_scheme(&url, "origin override")?;
            policy.upsert_override(origin.as_ref(), url);
        }
        Ok(policy)
    }

    pub fn with_override(mut self, origin: impl AsRef<str>, raw_url: String) -> Result<Self> {
        self.override_origin_str(origin.as_ref(), raw_url)?;
        Ok(self)
    }

    pub fn override_origin(&mut self, origin: impl AsRef<str>, raw_url: String) -> Result<()> {
        self.override_origin_str(origin.as_ref(), raw_url)
    }

    /// Layers `overrides` on top of the current policy.
    pub fn merged_with(&self, overrides: &Self) -> Self {
        let mut merged = self.clone();
        for (origin, url) in &overrides.overrides {
            merged.upsert_override(origin, url.clone());
        }
        merged
    }

    pub fn base_url(&self, origin: impl AsRef<str>) -> Result<Url> {
        self.base_url_str(origin.as_ref())
    }

    pub fn resolve(&self, origin: impl AsRef<str>, path: &str) -> Result<String> {
        self.resolve_str(origin.as_ref(), path)
    }

    pub fn resolve_segments(&self, origin: impl AsRef<str>, segments: &[&str]) -> Result<String> {
        self.resolve_segments_str(origin.as_ref(), segments)
    }

    pub fn override_for(&self, canonical: &str) -> Option<&Url> {
        let key = trim_trailing_slash(canonical);
        self.overrides
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, url)| url)
    }

    fn upsert_override(&mut self, canonical: &str, url: Url) {
        let key = trim_trailing_slash(canonical);
        if let Some((_, existing)) = self.overrides.iter_mut().find(|(stored, _)| stored == key) {
            *existing = url;
        } else {
            self.overrides.push((key.to_owned(), url));
        }
    }

    fn override_origin_str(&mut self, canonical: &str, raw_url: String) -> Result<()> {
        let url = Url::parse(&raw_url).map_err(move |source| {
            OriginError::invalid_url("parse origin override failed", raw_url, source)
        })?;
        require_http_scheme(&url, "origin override")?;
        self.upsert_override(canonical, url);
        Ok(())
    }

    fn base_url_str(&self, canonical: &str) -> Result<Url> {
        let url = match self.override_for(canonical) {
            Some(url) => url.clone(),
            None => Url::parse(canonical).map_err(|source| {
                OriginError::invalid_url("parse canonical origin failed", canonical, source)
            })?,
        };

        with_directory_path(url)
    }

    fn resolve_str(&self, canonical: &str, path: &str) -> Result<String> {
        let base = self.base_url_str(canonical)?;
        let suffix = path.trim_start_matches('/');
        if suffix.is_empty() {
            return Ok(trim_trailing_slash(base.as_str()).to_owned());
        }
        let suffix = sanitize_relative_path(suffix)?;

        base.join(&suffix)
            .map_err(|source| {
                OriginError::join_url("resolve origin path failed", base.as_str(), path, source)
            })
            .map(|url| url.to_string())
    }

    fn resolve_segments_str(&self, canonical: &str, segments: &[&str]) -> Result<String> {
        let mut url = self.base_url_str(canonical)?;
        let raw_url = url.to_string();
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|()| OriginError::UrlNotPathBase { url: raw_url })?;
            for segment in segments {
                path_segments.push(segment);
            }
        }

        Ok(url.to_string())
    }
}
