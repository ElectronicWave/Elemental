use std::{collections::HashMap, fmt::Debug, hash::Hash};

use elemental_infra::SafeRelativePath;
use reqwest::Url;

use crate::error::{Error, Result};

pub trait Origin: Copy + Debug + Eq + Hash + Send + Sync + 'static {
    fn canonical(self) -> &'static str;
    fn all() -> &'static [Self];
}

#[derive(Debug, Clone)]
pub struct OriginEndpoints<O: Origin> {
    origin_policy: OriginPolicy<O>,
}

impl<O: Origin> Default for OriginEndpoints<O> {
    fn default() -> Self {
        Self::official()
    }
}

impl<O: Origin> OriginEndpoints<O> {
    pub fn new(origin_policy: OriginPolicy<O>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn with_overrides<const N: usize>(overrides: [(O, String); N]) -> Result<Self> {
        let mut policy = OriginPolicy::default();
        for (origin, raw_url) in overrides {
            policy = policy.with_override(origin, raw_url)?;
        }
        Ok(Self::new(policy))
    }

    pub fn origin_policy(&self) -> &OriginPolicy<O> {
        &self.origin_policy
    }

    pub fn resolve(&self, origin: O, path: &str) -> Result<String> {
        self.origin_policy.resolve(origin, path)
    }

    pub fn resolve_segments<const N: usize>(
        &self,
        origin: O,
        segments: [&str; N],
    ) -> Result<String> {
        self.origin_policy.resolve_segments(origin, segments)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_origin_url(raw_url)
    }

    pub fn rewrite_known_or_keep(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_known_origin_url_or_keep(raw_url)
    }
}

#[derive(Debug, Clone)]
pub struct OriginPolicy<O: Origin> {
    overrides: HashMap<O, Url>,
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
            ensure_http_url(url, "origin override")?;
        }
        Ok(Self { overrides })
    }

    pub fn with_override(mut self, origin: O, raw_url: String) -> Result<Self> {
        let url = Url::parse(&raw_url).map_err(move |source| {
            Error::invalid_url("parse origin override failed", raw_url, source)
        })?;
        ensure_http_url(&url, "origin override")?;
        self.overrides.insert(origin, url);
        Ok(self)
    }

    pub fn base_url(&self, origin: O) -> Result<Url> {
        let url = match self.overrides.get(&origin) {
            Some(url) => url.clone(),
            None => Url::parse(origin.canonical()).map_err(|source| {
                Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
            })?,
        };

        ensure_directory_url(url)
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

    pub fn rewrite_origin_url(&self, raw_url: &str) -> Result<String> {
        let Some(rewritten) = self.try_rewrite_origin_url(raw_url)? else {
            return Err(Error::UnknownOrigin {
                raw_url: raw_url.to_owned(),
            });
        };

        Ok(rewritten)
    }

    pub fn try_rewrite_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| Error::invalid_url("parse url failed", raw_url, source))?;

        for origin in O::all() {
            let canonical = Url::parse(origin.canonical()).map_err(|source| {
                Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
            })?;
            if let Some(suffix) = origin_suffix(&parsed, &canonical) {
                return self.resolve(*origin, &suffix).map(Some);
            }

            if let Some(override_base) = self.overrides.get(origin)
                && let Some(suffix) = origin_suffix(&parsed, override_base)
            {
                return self.resolve(*origin, &suffix).map(Some);
            }
        }

        Ok(None)
    }

    pub fn rewrite_known_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| Error::invalid_url("parse url failed", raw_url, source))?;
        let mut matched_known_origin = false;

        for origin in O::all() {
            let canonical = Url::parse(origin.canonical()).map_err(|source| {
                Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
            })?;

            if same_origin(&parsed, &canonical) {
                matched_known_origin = true;
                if let Some(suffix) = origin_suffix(&parsed, &canonical) {
                    return self.resolve(*origin, &suffix).map(Some);
                }
            }

            if let Some(override_base) = self.overrides.get(origin)
                && same_origin(&parsed, override_base)
            {
                matched_known_origin = true;
                if let Some(suffix) = origin_suffix(&parsed, override_base) {
                    return self.resolve(*origin, &suffix).map(Some);
                }
            }
        }

        if matched_known_origin {
            return Err(Error::OriginBasePathMismatch {
                raw_url: raw_url.to_owned(),
            });
        }

        Ok(None)
    }

    pub fn rewrite_known_origin_url_or_keep(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}

fn ensure_http_url(url: &Url, context: &str) -> Result<()> {
    if matches!(url.scheme(), "http" | "https") {
        return Ok(());
    }

    Err(Error::UnsupportedUrlScheme {
        context: context.to_owned(),
        url: url.to_string(),
    })
}

pub(crate) fn ensure_directory_url(mut url: Url) -> Result<Url> {
    if !url.path().ends_with('/') {
        let raw_url = url.to_string();
        url.path_segments_mut()
            .map_err(|()| Error::UrlNotPathBase { url: raw_url })?
            .push("");
    }

    Ok(url)
}

fn trim_trailing_slash(raw_url: &str) -> &str {
    raw_url.trim_end_matches('/')
}

fn origin_suffix(raw_url: &Url, base_url: &Url) -> Option<String> {
    if !same_origin(raw_url, base_url) {
        return None;
    }

    let raw = raw_url.as_str();
    let base = trim_trailing_slash(base_url.as_str());
    let suffix = raw.strip_prefix(base)?;
    if base_url.path().ends_with('/') || suffix.is_empty() || suffix.starts_with('/') {
        return Some(suffix.to_owned());
    }

    None
}

fn same_origin(raw_url: &Url, base_url: &Url) -> bool {
    raw_url.scheme() == base_url.scheme()
        && raw_url.host_str() == base_url.host_str()
        && raw_url.port_or_known_default() == base_url.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::{Origin, OriginPolicy};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestOrigin {
        Base,
    }

    impl Origin for TestOrigin {
        fn canonical(self) -> &'static str {
            "https://example.com/releases"
        }

        fn all() -> &'static [Self] {
            &[Self::Base]
        }
    }

    #[test]
    fn resolve_preserves_base_path_without_trailing_slash() {
        let resolved = OriginPolicy::default()
            .resolve(TestOrigin::Base, "org/example/a.jar")
            .expect("base path should resolve");

        assert_eq!(resolved, "https://example.com/releases/org/example/a.jar");
    }

    #[test]
    fn origin_suffix_requires_a_path_boundary() {
        let policy: OriginPolicy<TestOrigin> = OriginPolicy::default();
        let rewritten = policy
            .try_rewrite_origin_url("https://example.com/releases-v2/a.jar")
            .expect("url should parse");

        assert!(rewritten.is_none());
    }
}
