use std::collections::HashMap;

use elemental_infra::SafeRelativePath;
use reqwest::Url;

use super::Origin;
use super::util::{require_http_scheme, trim_trailing_slash, with_directory_path};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct OriginPolicy<O: Origin> {
    pub(super) overrides: HashMap<O, Url>,
}

impl<O: Origin> Default for OriginPolicy<O> {
    fn default() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }
}

impl<O: Origin> OriginPolicy<O> {
    pub fn new(overrides: HashMap<O, Url>) -> Result<Self> {
        for url in overrides.values() {
            require_http_scheme(url, "origin override")?;
        }
        Ok(Self { overrides })
    }

    pub fn with_override(mut self, origin: O, raw_url: String) -> Result<Self> {
        self.override_origin(origin, raw_url)?;
        Ok(self)
    }

    pub fn override_origin(&mut self, origin: O, raw_url: String) -> Result<()> {
        let url = Url::parse(&raw_url).map_err(move |source| {
            Error::invalid_url("parse origin override failed", raw_url, source)
        })?;
        require_http_scheme(&url, "origin override")?;
        self.overrides.insert(origin, url);
        Ok(())
    }

    /// Layers `overrides` on top of the current policy.
    pub fn merged_with(&self, overrides: &Self) -> Self {
        let mut merged = self.clone();
        merged.overrides.extend(
            overrides
                .overrides
                .iter()
                .map(|(origin, url)| (*origin, url.clone())),
        );
        merged
    }

    pub fn base_url(&self, origin: O) -> Result<Url> {
        let url = match self.overrides.get(&origin) {
            Some(url) => url.clone(),
            None => Url::parse(origin.canonical()).map_err(|source| {
                Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
            })?,
        };

        with_directory_path(url)
    }

    pub fn resolve(&self, origin: O, path: &str) -> Result<String> {
        let base = self.base_url(origin)?;
        let suffix = path.trim_start_matches('/');
        if suffix.is_empty() {
            return Ok(trim_trailing_slash(base.as_str()).to_owned());
        }
        let suffix = SafeRelativePath::parse(suffix)?;

        base.join(suffix.as_str())
            .map_err(|source| {
                Error::join_url("resolve origin path failed", base.as_str(), path, source)
            })
            .map(|url| url.to_string())
    }

    pub fn resolve_segments<const N: usize>(
        &self,
        origin: O,
        segments: [&str; N],
    ) -> Result<String> {
        let mut url = self.base_url(origin)?;
        let raw_url = url.to_string();
        {
            let mut path_segments = url
                .path_segments_mut()
                .map_err(|()| Error::UrlNotPathBase { url: raw_url })?;
            for segment in segments {
                path_segments.push(segment);
            }
        }

        Ok(url.to_string())
    }
}
