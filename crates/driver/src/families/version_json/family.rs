use std::fmt::Debug;

use crate::error::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::fabric::ProfileJson;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    driver::{DriverDescriptor, InstalledDriver},
    drivers::vanilla::source::VanillaSource,
    families::version_json::{PASSTHROUGH_PROFILE_BEHAVIOR, merge_profile_with_behavior},
    inspect::ProfiledDriverIdentity,
    loader_version::LoaderVersionId,
};

use super::{VersionJsonRemoteResolver, driver::ProfiledVersionJsonDriver};

#[async_trait(?Send)]
pub trait ProfiledVersionJsonFamily: Clone + Debug + Send + Sync + 'static {
    type Source: Clone + Debug + Send + Sync + 'static;
    type RemoteResolver: VersionJsonRemoteResolver + Clone + Send + Sync + 'static;

    fn descriptor(&self) -> DriverDescriptor;

    fn identity(&self) -> Option<ProfiledDriverIdentity> {
        None
    }

    fn default_source(&self) -> Result<Self::Source>
    where
        Self::Source: Default,
    {
        Ok(Self::Source::default())
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver;

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<ProfileJson>;

    fn merge_profile(
        &self,
        base_metadata: PistonMetaData,
        profile: ProfileJson,
    ) -> Result<PistonMetaData> {
        merge_profile_with_behavior(&PASSTHROUGH_PROFILE_BEHAVIOR, base_metadata, profile)
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        self.identity().is_some_and(|identity| {
            identity.local_metadata_needs_refresh(metadata, game_version, loader_version.as_str())
        })
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        self.identity()
            .and_then(|identity| identity.inspect_installed(metadata))
    }
}

pub trait ProfiledVersionJsonFamilyExt: ProfiledVersionJsonFamily + Sized {
    fn build_driver(self, source: Self::Source) -> ProfiledVersionJsonDriver<Self> {
        ProfiledVersionJsonDriver::new(self, source)
    }

    fn build_driver_with_defaults(self) -> Result<ProfiledVersionJsonDriver<Self>>
    where
        Self::Source: Default,
    {
        ProfiledVersionJsonDriver::with_defaults(self)
    }
}

impl<F> ProfiledVersionJsonFamilyExt for F where F: ProfiledVersionJsonFamily {}
