use std::fmt::Debug;

mod error;
mod known;
mod map;
mod policy;
mod rewrite;
#[cfg(test)]
mod tests;
mod util;

pub use error::{OriginError, Result};
pub use map::OriginPolicyMap;
pub use policy::OriginPolicy;
pub use util::{require_http_scheme, with_directory_path};

pub trait Origin: Copy + Debug + Send + Sync + 'static {
    fn canonical(self) -> &'static str;
    fn all() -> &'static [Self];
}

#[derive(Debug, Clone)]
pub struct OriginEndpoints {
    known_origins: &'static [&'static str],
    origin_policy: OriginPolicy,
}

impl OriginEndpoints {
    pub fn new(known_origins: &'static [&'static str], origin_policy: OriginPolicy) -> Self {
        Self {
            known_origins,
            origin_policy,
        }
    }

    pub fn official(known_origins: &'static [&'static str]) -> Self {
        Self::new(known_origins, OriginPolicy::default())
    }

    pub fn with_overrides(
        known_origins: &'static [&'static str],
        overrides: impl IntoIterator<Item = (impl AsRef<str>, String)>,
    ) -> Result<Self> {
        let mut policy = OriginPolicy::default();
        for (origin, raw_url) in overrides {
            policy.override_origin(origin, raw_url)?;
        }
        Ok(Self::new(known_origins, policy))
    }

    pub fn origin_policy(&self) -> &OriginPolicy {
        &self.origin_policy
    }

    pub fn known_origins(&self) -> &'static [&'static str] {
        self.known_origins
    }

    pub fn resolve(&self, origin: impl AsRef<str>, path: &str) -> Result<String> {
        self.origin_policy.resolve(origin, path)
    }

    pub fn resolve_segments(&self, origin: impl AsRef<str>, segments: &[&str]) -> Result<String> {
        self.origin_policy.resolve_segments(origin, segments)
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy
            .rewrite_origin_url(self.known_origins, raw_url)
    }

    pub fn rewrite_known_or_keep(&self, raw_url: &str) -> Result<String> {
        self.origin_policy
            .rewrite_known_origin_url_or_keep(self.known_origins, raw_url)
    }
}
