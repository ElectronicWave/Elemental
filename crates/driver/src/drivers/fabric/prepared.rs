use crate::{
    drivers::fabric::source::FabricEndpoints,
    families::version_json::{
        LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
        ResolvedVersionJsonMetadata, VanillaFallbackRemoteResolver, VersionJsonInstallStatus,
    },
};

pub type FabricRemoteResolver = VanillaFallbackRemoteResolver<FabricEndpoints>;
pub type ResolvedFabricMetadata = ResolvedVersionJsonMetadata<FabricRemoteResolver>;
pub type FabricInstallStatus = VersionJsonInstallStatus;
pub type ResolvedFabricVersion<L, VL> = ResolvedVersionJsonInstance<FabricRemoteResolver, L, VL>;
pub type PreparedFabricVersion<L, VL> = PreparedVersionJsonInstance<FabricRemoteResolver, L, VL>;
pub type LaunchedFabricVersion<L, VL> = LaunchedVersionJsonInstance<FabricRemoteResolver, L, VL>;
