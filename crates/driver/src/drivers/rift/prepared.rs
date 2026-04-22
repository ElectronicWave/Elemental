use anyhow::Result;

use crate::drivers::{
    rift::source::RiftEndpoints,
    vanilla::source::{VanillaEndpoints, rewrite_upstream_with_vanilla_fallback},
};
use crate::families::version_json::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
    ResolvedVersionJsonMetadata, VersionJsonInstallStatus, VersionJsonRemoteResolver,
};

#[derive(Debug, Clone)]
pub struct RiftRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    rift_endpoints: RiftEndpoints,
}

pub type ResolvedRiftMetadata = ResolvedVersionJsonMetadata<RiftRemoteResolver>;
pub type RiftInstallStatus = VersionJsonInstallStatus;
pub type ResolvedRiftVersion<L, VL> = ResolvedVersionJsonInstance<RiftRemoteResolver, L, VL>;
pub type PreparedRiftVersion<L, VL> = PreparedVersionJsonInstance<RiftRemoteResolver, L, VL>;
pub type LaunchedRiftVersion<L, VL> = LaunchedVersionJsonInstance<RiftRemoteResolver, L, VL>;

impl RiftRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, rift_endpoints: RiftEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            rift_endpoints,
        }
    }
}

impl VersionJsonRemoteResolver for RiftRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(&self.vanilla_endpoints, raw_url, "rift", || {
            self.rift_endpoints.rewrite_upstream(raw_url)
        })
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
