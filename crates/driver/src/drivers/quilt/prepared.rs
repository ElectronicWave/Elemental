use crate::{
    drivers::quilt::source::QuiltEndpoints, families::version_json::VanillaFallbackRemoteResolver,
};

pub type QuiltRemoteResolver = VanillaFallbackRemoteResolver<QuiltEndpoints>;
