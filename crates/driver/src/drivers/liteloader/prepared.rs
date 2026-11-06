use crate::{
    drivers::liteloader::source::LiteLoaderEndpoints,
    families::version_json::{PreparedVersionJsonInstance, VanillaFallbackRemoteResolver},
};

pub type LiteLoaderRemoteResolver = VanillaFallbackRemoteResolver<LiteLoaderEndpoints>;
pub type PreparedLiteLoaderVersion<L, VL> =
    PreparedVersionJsonInstance<LiteLoaderRemoteResolver, L, VL>;
