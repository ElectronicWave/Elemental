use crate::{
    drivers::rift::source::RiftEndpoints,
    families::version_json::{
        LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
        ResolvedVersionJsonMetadata, VanillaFallbackRemoteResolver, VersionJsonInstallStatus,
    },
};

pub type RiftRemoteResolver = VanillaFallbackRemoteResolver<RiftEndpoints>;
pub type ResolvedRiftMetadata = ResolvedVersionJsonMetadata<RiftRemoteResolver>;
pub type RiftInstallStatus = VersionJsonInstallStatus;
pub type ResolvedRiftVersion<L, VL> = ResolvedVersionJsonInstance<RiftRemoteResolver, L, VL>;
pub type PreparedRiftVersion<L, VL> = PreparedVersionJsonInstance<RiftRemoteResolver, L, VL>;
pub type LaunchedRiftVersion<L, VL> = LaunchedVersionJsonInstance<RiftRemoteResolver, L, VL>;
