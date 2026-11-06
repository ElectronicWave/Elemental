use crate::error::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::{mojang::piston::PistonMetaData, quilt::ProfileJson};

use crate::{
    descriptors::QUILT_DRIVER,
    driver::{DriverDescriptor, InstalledDriver},
    drivers::{quilt::source::QuiltSource, vanilla::source::VanillaSource},
    families::version_json::{
        ProfiledVersionJsonDriver, ProfiledVersionJsonFamily, VanillaFallbackRemoteResolver,
    },
    inspect::LibraryPrefixSet,
    loader_version::LoaderVersionId,
};

const QUILT_LOADER_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["org.quiltmc:quilt-loader:"]);

#[derive(Debug, Clone, Copy, Default)]
pub struct QuiltDriverFamily;

#[async_trait(?Send)]
impl ProfiledVersionJsonFamily for QuiltDriverFamily {
    type Source = QuiltSource;
    type RemoteResolver = super::prepared::QuiltRemoteResolver;

    fn descriptor(&self) -> DriverDescriptor {
        QUILT_DRIVER
    }

    fn remote_resolver(
        &self,
        vanilla_source: &VanillaSource,
        source: &Self::Source,
    ) -> Self::RemoteResolver {
        VanillaFallbackRemoteResolver::new(
            "quilt",
            vanilla_source.endpoints().clone(),
            source.endpoints().clone(),
        )
    }

    async fn profile(
        &self,
        source: &Self::Source,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> Result<ProfileJson> {
        source
            .profile_json(game_version.as_str(), loader_version.as_str())
            .await
    }

    fn local_metadata_needs_refresh(
        &self,
        metadata: &PistonMetaData,
        game_version: &MinecraftVersionId,
        loader_version: &LoaderVersionId,
    ) -> bool {
        let expected_id = format!("quilt-loader-{loader_version}-{game_version}");
        metadata.id != expected_id
            || metadata.inherits_from.as_deref() != Some(game_version.as_str())
            || QUILT_LOADER_LIBRARIES
                .version(metadata)
                .is_none_or(|installed| installed != loader_version.as_str())
    }

    fn inspect_installed(&self, metadata: &PistonMetaData) -> Option<InstalledDriver> {
        QUILT_LOADER_LIBRARIES.installed_driver(metadata, QUILT_DRIVER)
    }
}

pub type QuiltDriver = ProfiledVersionJsonDriver<QuiltDriverFamily>;
