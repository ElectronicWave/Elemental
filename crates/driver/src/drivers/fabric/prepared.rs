use crate::{
    drivers::fabric::source::FabricEndpoints, families::version_json::VanillaFallbackRemoteResolver,
};

pub type FabricRemoteResolver = VanillaFallbackRemoteResolver<FabricEndpoints>;
