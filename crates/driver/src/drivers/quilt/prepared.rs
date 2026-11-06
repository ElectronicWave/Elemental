use crate::{
    drivers::quilt::source::QuiltEndpoints,
    families::version_json::{PreparedVersionJsonInstance, VanillaFallbackRemoteResolver},
};

pub type QuiltRemoteResolver = VanillaFallbackRemoteResolver<QuiltEndpoints>;
pub type PreparedQuiltVersion<L, VL> = PreparedVersionJsonInstance<QuiltRemoteResolver, L, VL>;
