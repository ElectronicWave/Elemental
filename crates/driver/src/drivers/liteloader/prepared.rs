use crate::{
    drivers::liteloader::source::LiteLoaderEndpoints,
    families::version_json::{
        LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
        ResolvedVersionJsonMetadata, VanillaFallbackRemoteResolver, VersionJsonInstallStatus,
    },
};

pub type LiteLoaderRemoteResolver = VanillaFallbackRemoteResolver<LiteLoaderEndpoints>;
pub type ResolvedLiteLoaderMetadata = ResolvedVersionJsonMetadata<LiteLoaderRemoteResolver>;
pub type LiteLoaderInstallStatus = VersionJsonInstallStatus;
pub type ResolvedLiteLoaderVersion<L, VL> =
    ResolvedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;
pub type PreparedLiteLoaderVersion<L, VL> =
    PreparedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;
pub type LaunchedLiteLoaderVersion<L, VL> =
    LaunchedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;
