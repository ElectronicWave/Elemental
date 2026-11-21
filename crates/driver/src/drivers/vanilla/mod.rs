use crate::families::version_json::{ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata};

pub mod catalog;
pub mod driver;
pub mod endpoints;
pub mod source;

pub type ResolvedVanillaMetadata = ResolvedVersionJsonMetadata;
pub type ResolvedVanillaVersion = ResolvedVersionJsonInstance;
