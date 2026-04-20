use std::{fmt::Debug, sync::Arc};

use anyhow::{Context, Result};
use reqwest::Url;

pub trait UrlRule: Debug + Send + Sync {
    fn apply(&self, url: Url) -> Result<Url>;
}

#[derive(Debug, Clone, Default)]
pub struct UrlMapper {
    rules: Vec<Arc<dyn UrlRule>>,
}

#[derive(Debug, Clone)]
pub struct ReplaceHostRule {
    from: String,
    to: String,
}

#[derive(Debug, Clone)]
pub struct ReplacePrefixRule {
    from: String,
    to: String,
}

impl UrlMapper {
    pub fn new(rules: Vec<Arc<dyn UrlRule>>) -> Self {
        Self { rules }
    }

    pub fn add_rule<R>(mut self, rule: R) -> Self
    where
        R: UrlRule + 'static,
    {
        self.rules.push(Arc::new(rule));
        self
    }

    pub fn rewrite(&self, raw_url: impl AsRef<str>) -> Result<String> {
        let url = Url::parse(raw_url.as_ref())
            .with_context(|| format!("parse url failed: {}", raw_url.as_ref()))?;
        Ok(self.rewrite_url(url)?.to_string())
    }

    pub fn rewrite_url(&self, mut url: Url) -> Result<Url> {
        for rule in &self.rules {
            url = rule.apply(url)?;
        }

        Ok(url)
    }
}

impl ReplaceHostRule {
    pub fn new(from: String, to: String) -> Self {
        Self { from, to }
    }
}

impl UrlRule for ReplaceHostRule {
    fn apply(&self, mut url: Url) -> Result<Url> {
        if url.host_str().is_some_and(|host| host == self.from) {
            url.set_host(Some(&self.to)).with_context(|| {
                format!("replace host '{}' with '{}' failed", self.from, self.to)
            })?;
        }

        Ok(url)
    }
}

impl ReplacePrefixRule {
    pub fn new(from: String, to: String) -> Self {
        Self { from, to }
    }
}

impl UrlRule for ReplacePrefixRule {
    fn apply(&self, url: Url) -> Result<Url> {
        let raw_url = url.to_string();
        if !raw_url.starts_with(&self.from) {
            return Ok(url);
        }

        Url::parse(&raw_url.replacen(&self.from, &self.to, 1)).with_context(|| {
            format!(
                "replace url prefix '{}' with '{}' failed: {}",
                self.from, self.to, raw_url
            )
        })
    }
}
