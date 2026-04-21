use anyhow::Result;

use crate::drivers::{
    fabric::source::FabricEndpoints,
    vanilla::source::{VanillaEndpoints, rewrite_upstream_with_vanilla_fallback},
};
use crate::families::version_json::{
    LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
    ResolvedVersionJsonMetadata, VersionJsonInstallStatus, VersionJsonRemoteResolver,
};

#[derive(Debug, Clone)]
pub struct FabricRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    fabric_endpoints: FabricEndpoints,
}

pub type ResolvedFabricMetadata = ResolvedVersionJsonMetadata<FabricRemoteResolver>;
pub type FabricInstallStatus = VersionJsonInstallStatus;
pub type ResolvedFabricVersion<L, VL> = ResolvedVersionJsonInstance<FabricRemoteResolver, L, VL>;
pub type PreparedFabricVersion<L, VL> = PreparedVersionJsonInstance<FabricRemoteResolver, L, VL>;
pub type LaunchedFabricVersion<L, VL> = LaunchedVersionJsonInstance<FabricRemoteResolver, L, VL>;

impl FabricRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, fabric_endpoints: FabricEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            fabric_endpoints,
        }
    }
}

impl VersionJsonRemoteResolver for FabricRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(&self.vanilla_endpoints, raw_url, "fabric", || {
            self.fabric_endpoints.rewrite_upstream(raw_url)
        })
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
