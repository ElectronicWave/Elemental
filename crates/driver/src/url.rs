use std::{fmt::Debug, hash::Hash};

use crate::error::Result;

mod known;
mod policy;
mod rewrite;
#[cfg(test)]
mod tests;
mod util;

pub use policy::OriginPolicy;
pub(crate) use util::{require_http_scheme, with_directory_path};

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
