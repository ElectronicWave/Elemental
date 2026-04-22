use crate::{
    drivers::quilt::source::QuiltEndpoints,
    families::version_json::{
        LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
        ResolvedVersionJsonMetadata, VanillaFallbackRemoteResolver, VersionJsonInstallStatus,
    },
};

pub type QuiltRemoteResolver = VanillaFallbackRemoteResolver<QuiltEndpoints>;
pub type ResolvedQuiltMetadata = ResolvedVersionJsonMetadata<QuiltRemoteResolver>;
pub type QuiltInstallStatus = VersionJsonInstallStatus;
pub type ResolvedQuiltVersion<L, VL> = ResolvedVersionJsonInstance<QuiltRemoteResolver, L, VL>;
pub type PreparedQuiltVersion<L, VL> = PreparedVersionJsonInstance<QuiltRemoteResolver, L, VL>;
pub type LaunchedQuiltVersion<L, VL> = LaunchedVersionJsonInstance<QuiltRemoteResolver, L, VL>;
