use anyhow::Result;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{fabric::ProfileJson, mojang::piston::PistonMetaData};

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::vanilla::source::VanillaSource,
    inspect::ProfiledDriverIdentity,
    loader_version::LoaderVersionId,
};

use super::{
    PASSTHROUGH_PROFILE_BEHAVIOR, UpstreamUrlRewriter, VanillaFallbackRemoteResolver,
    merge_profile_with_behavior,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectProfiledVersionJsonDefaults {
    descriptor: DriverDescriptor,
    family_name: &'static str,
    identity: ProfiledDriverIdentity,
}

impl DirectProfiledVersionJsonDefaults {
    pub const fn new(
        descriptor: DriverDescriptor,
        family_name: &'static str,
        identity: ProfiledDriverIdentity,
    ) -> Self {
        Self {
            descriptor,
            family_name,
            identity,
        }
    }

    pub fn descriptor(self) -> DriverDescriptor {
        self.descriptor
    }

    pub fn default_source<S>(self) -> Result<S>
    where
        S: Default,
    {
        Ok(S::default())
    }

    pub fn remote_resolver<E>(
        self,
        vanilla_source: &VanillaSource,
        family_endpoints: &E,
    ) -> VanillaFallbackRemoteResolver<E>
    where
        E: UpstreamUrlRewriter,
    {
        VanillaFallbackRemoteResolver::new(
            self.family_name,
            vanilla_source.endpoints().clone(),
            family_endpoints.clone(),
        )
    }

    pub fn merge_profile(
        self,
        base_metadata: PistonMetaData,
        profile: ProfileJson,
    ) -> Result<PistonMetaData> {
        merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
    }

    pub fn local_metadata_needs_refresh(
        self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        self.identity
            .local_metadata_needs_refresh(metadata, game_version, loader_version.as_str())
    }

    pub fn inspect_installed(self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        self.identity.inspect_installed(metadata)
    }
}
