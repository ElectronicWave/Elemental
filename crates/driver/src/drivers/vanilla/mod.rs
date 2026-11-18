use crate::families::version_json::{ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata};

use self::endpoints::VanillaEndpoints;

pub mod catalog;
pub mod driver;
pub mod endpoints;
pub mod source;

pub type ResolvedVanillaMetadata = ResolvedVersionJsonMetadata<VanillaEndpoints>;
pub type ResolvedVanillaVersion<L, VL> = ResolvedVersionJsonInstance<VanillaEndpoints, L, VL>;
