use anyhow::Result;

use crate::drivers::{
    quilt::source::QuiltEndpoints,
    vanilla::source::{VanillaEndpoints, rewrite_upstream_with_vanilla_fallback},
};
use crate::families::version_json::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
    ResolvedVersionJsonMetadata, VersionJsonInstallStatus, VersionJsonRemoteResolver,
};

#[derive(Debug, Clone)]
pub struct QuiltRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    quilt_endpoints: QuiltEndpoints,
}

pub type ResolvedQuiltMetadata = ResolvedVersionJsonMetadata<QuiltRemoteResolver>;
pub type QuiltInstallStatus = VersionJsonInstallStatus;
pub type ResolvedQuiltVersion<L, VL> = ResolvedVersionJsonInstance<QuiltRemoteResolver, L, VL>;
pub type PreparedQuiltVersion<L, VL> = PreparedVersionJsonInstance<QuiltRemoteResolver, L, VL>;
pub type LaunchedQuiltVersion<L, VL> = LaunchedVersionJsonInstance<QuiltRemoteResolver, L, VL>;

impl QuiltRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, quilt_endpoints: QuiltEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            quilt_endpoints,
        }
    }
}

impl VersionJsonRemoteResolver for QuiltRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(&self.vanilla_endpoints, raw_url, "quilt", || {
            self.quilt_endpoints.rewrite_upstream(raw_url)
        })
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
