use crate::{
    drivers::fabric::source::FabricEndpoints,
    families::version_json::{PreparedVersionJsonInstance, VanillaFallbackRemoteResolver},
};

pub type FabricRemoteResolver = VanillaFallbackRemoteResolver<FabricEndpoints>;
pub type PreparedFabricVersion<L, VL> = PreparedVersionJsonInstance<FabricRemoteResolver, L, VL>;
