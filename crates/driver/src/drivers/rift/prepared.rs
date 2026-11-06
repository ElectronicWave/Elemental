use crate::{
    drivers::rift::source::RiftEndpoints,
    families::version_json::{PreparedVersionJsonInstance, VanillaFallbackRemoteResolver},
};

pub type RiftRemoteResolver = VanillaFallbackRemoteResolver<RiftEndpoints>;
pub type PreparedRiftVersion<L, VL> = PreparedVersionJsonInstance<RiftRemoteResolver, L, VL>;
