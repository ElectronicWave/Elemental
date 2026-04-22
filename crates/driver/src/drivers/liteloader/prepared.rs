use anyhow::Result;

use crate::drivers::{
    liteloader::source::LiteLoaderEndpoints,
    vanilla::source::{VanillaEndpoints, rewrite_upstream_with_vanilla_fallback},
};
use crate::families::version_json::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
    ResolvedVersionJsonMetadata, VersionJsonInstallStatus, VersionJsonRemoteResolver,
};

#[derive(Debug, Clone)]
pub struct LiteLoaderRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    liteloader_endpoints: LiteLoaderEndpoints,
}

pub type ResolvedLiteLoaderMetadata = ResolvedVersionJsonMetadata<LiteLoaderRemoteResolver>;
pub type LiteLoaderInstallStatus = VersionJsonInstallStatus;
pub type ResolvedLiteLoaderVersion<L, VL> =
    ResolvedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;
pub type PreparedLiteLoaderVersion<L, VL> =
    PreparedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;
pub type LaunchedLiteLoaderVersion<L, VL> =
    LaunchedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;

impl LiteLoaderRemoteResolver {
    pub fn new(
        vanilla_endpoints: VanillaEndpoints,
        liteloader_endpoints: LiteLoaderEndpoints,
    ) -> Self {
        Self {
            vanilla_endpoints,
            liteloader_endpoints,
        }
    }
}

impl VersionJsonRemoteResolver for LiteLoaderRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(
            &self.vanilla_endpoints,
            raw_url,
            "liteloader",
            || self.liteloader_endpoints.rewrite_upstream(raw_url),
        )
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
