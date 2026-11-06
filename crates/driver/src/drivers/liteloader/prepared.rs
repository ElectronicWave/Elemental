use crate::{
    drivers::liteloader::source::LiteLoaderEndpoints,
    families::version_json::VanillaFallbackRemoteResolver,
};

pub type LiteLoaderRemoteResolver = VanillaFallbackRemoteResolver<LiteLoaderEndpoints>;
