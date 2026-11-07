use crate::drivers::vanilla::source::VanillaEndpoints;
use crate::families::version_json::{ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata};

pub type ResolvedVanillaMetadata = ResolvedVersionJsonMetadata<VanillaEndpoints>;
pub type ResolvedVanillaVersion<L, VL> = ResolvedVersionJsonInstance<VanillaEndpoints, L, VL>;
