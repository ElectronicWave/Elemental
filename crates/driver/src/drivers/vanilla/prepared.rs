use crate::drivers::{
    vanilla::source::VanillaEndpoints,
    version_json::{
        LaunchedVersionJsonInstance, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
        ResolvedVersionJsonMetadata, VersionJsonInstallStatus, prepared::VersionJsonInstallPlanner,
    },
};

pub type ResolvedVanillaMetadata = ResolvedVersionJsonMetadata<VanillaEndpoints>;
pub type VanillaInstallStatus = VersionJsonInstallStatus;
pub type ResolvedVanillaVersion<L, VL> = ResolvedVersionJsonInstance<VanillaEndpoints, L, VL>;
pub type PreparedVanillaVersion<L, VL> = PreparedVersionJsonInstance<VanillaEndpoints, L, VL>;
pub type LaunchedVanillaVersion<L, VL> = LaunchedVersionJsonInstance<VanillaEndpoints, L, VL>;
pub type VanillaInstallPlanner<'a, L, VL> = VersionJsonInstallPlanner<'a, VanillaEndpoints, L, VL>;
