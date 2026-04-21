use std::{collections::HashMap, fmt::Debug, hash::Hash};

use anyhow::{Context, Result, bail};
use reqwest::Url;

pub trait Origin: Copy + Debug + Eq + Hash + Send + Sync + 'static {
    fn canonical(self) -> &'static str;
    fn all() -> &'static [Self];
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
    pub fn new(overrides: HashMap<O, Url>) -> Self {
        Self { overrides }
    }

    pub fn with_override(mut self, origin: O, raw_url: String) -> Result<Self> {
        let url = Url::parse(&raw_url)
            .with_context(|| format!("parse origin override failed: {raw_url}"))?;
        self.overrides.insert(origin, url);
        Ok(self)
    }

    pub fn base_url(&self, origin: O) -> Result<Url> {
        if let Some(url) = self.overrides.get(&origin) {
            return Ok(url.clone());
        }

        Url::parse(origin.canonical())
            .with_context(|| format!("parse canonical origin failed: {}", origin.canonical()))
    }

    pub fn resolve(&self, origin: O, path: &str) -> Result<String> {
        let base = self.base_url(origin)?;
        let suffix = path.trim_start_matches('/');
        if suffix.is_empty() {
            return Ok(trim_trailing_slash(base.as_str()).to_owned());
        }

        Url::parse(&format!(
            "{}/{}",
            trim_trailing_slash(base.as_str()),
            suffix
        ))
        .with_context(|| {
            format!(
                "resolve origin path failed for base '{}' and path '{}'",
                base, path
            )
        })
        .map(|url| url.to_string())
    }

    pub fn rewrite_origin_url(&self, raw_url: &str) -> Result<String> {
        let Some(rewritten) = self.try_rewrite_origin_url(raw_url)? else {
            bail!("can't map url to a known origin: {raw_url}")
        };

        Ok(rewritten)
    }

    pub fn try_rewrite_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url).with_context(|| format!("parse url failed: {raw_url}"))?;

        for origin in O::all() {
            let canonical = Url::parse(origin.canonical()).with_context(|| {
                format!("parse canonical origin failed: {}", origin.canonical())
            })?;
            if let Some(suffix) = origin_suffix(&parsed, &canonical) {
                return self.resolve(*origin, &suffix).map(Some);
            }

            if let Some(override_base) = self.overrides.get(origin) {
                if let Some(suffix) = origin_suffix(&parsed, override_base) {
                    return self.resolve(*origin, &suffix).map(Some);
                }
            }
        }

        Ok(None)
    }

    pub fn rewrite_known_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url).with_context(|| format!("parse url failed: {raw_url}"))?;
        let mut matched_known_origin = false;

        for origin in O::all() {
            let canonical = Url::parse(origin.canonical()).with_context(|| {
                format!("parse canonical origin failed: {}", origin.canonical())
            })?;

            if same_origin(&parsed, &canonical) {
                matched_known_origin = true;
                if let Some(suffix) = origin_suffix(&parsed, &canonical) {
                    return self.resolve(*origin, &suffix).map(Some);
                }
            }

            if let Some(override_base) = self.overrides.get(origin) {
                if same_origin(&parsed, override_base) {
                    matched_known_origin = true;
                    if let Some(suffix) = origin_suffix(&parsed, override_base) {
                        return self.resolve(*origin, &suffix).map(Some);
                    }
                }
            }
        }

        if matched_known_origin {
            bail!("url matches a known origin host but not a configured base path: {raw_url}");
        }

        Ok(None)
    }
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
    raw.strip_prefix(base).map(ToOwned::to_owned)
}

fn same_origin(raw_url: &Url, base_url: &Url) -> bool {
    raw_url.scheme() == base_url.scheme()
        && raw_url.host_str() == base_url.host_str()
        && raw_url.port_or_known_default() == base_url.port_or_known_default()
}
