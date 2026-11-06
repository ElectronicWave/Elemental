use crate::{
    drivers::rift::source::RiftEndpoints, families::version_json::VanillaFallbackRemoteResolver,
};

pub type RiftRemoteResolver = VanillaFallbackRemoteResolver<RiftEndpoints>;
