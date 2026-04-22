use std::fmt::Debug;

use anyhow::Result;

use crate::drivers::vanilla::source::{VanillaEndpoints, rewrite_upstream_with_vanilla_fallback};

pub trait VersionJsonRemoteResolver: Clone + Debug + Send + Sync + 'static {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String>;
    fn object_url(&self, hash: &str) -> Result<String>;
}

pub trait UpstreamUrlRewriter: Clone + Debug + Send + Sync + 'static {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    family_name: &'static str,
    vanilla_endpoints: VanillaEndpoints,
    family_endpoints: E,
}

impl<E> VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    pub fn new(
        family_name: &'static str,
        vanilla_endpoints: VanillaEndpoints,
        family_endpoints: E,
    ) -> Self {
        Self {
            family_name,
            vanilla_endpoints,
            family_endpoints,
        }
    }
}

impl<E> VersionJsonRemoteResolver for VanillaFallbackRemoteResolver<E>
where
    E: UpstreamUrlRewriter,
{
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(
            &self.vanilla_endpoints,
            raw_url,
            self.family_name,
            || self.family_endpoints.rewrite_upstream(raw_url),
        )
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
