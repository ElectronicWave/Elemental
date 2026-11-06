use std::path::Path;

use elemental_core::{
    auth::authorizer::Authorizer, launcher::command::LaunchCommand,
    runtime::distribution::Distribution, storage::Storage,
};
use elemental_infra::downloader::DownloadSession;

use crate::{
    descriptors::{
        BABRIC_DRIVER, FABRIC_DRIVER, LEGACY_FABRIC_DRIVER, LITELOADER_DRIVER, QUILT_DRIVER,
        RIFT_DRIVER,
    },
    driver::DriverDescriptor,
    drivers::{
        fabric::{driver::FabricDriver, prepared::PreparedFabricVersion},
        liteloader::{driver::LiteLoaderDriver, prepared::PreparedLiteLoaderVersion},
        quilt::{driver::QuiltDriver, prepared::PreparedQuiltVersion},
        rift::{driver::RiftDriver, prepared::PreparedRiftVersion},
    },
    error::{Error, Result},
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonLaunchConfig, VersionJsonRootLayout,
    },
};

use super::LoaderRequest;

pub enum ProfiledDriver {
    FabricLike(FabricDriver),
    Quilt(QuiltDriver),
    LiteLoader(LiteLoaderDriver),
    Rift(RiftDriver),
}

pub enum ProfiledPrepared<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    FabricLike(PreparedFabricVersion<L, VL>),
    Quilt(PreparedQuiltVersion<L, VL>),
    LiteLoader(PreparedLiteLoaderVersion<L, VL>),
    Rift(PreparedRiftVersion<L, VL>),
}

impl ProfiledDriver {
    pub fn descriptor(&self) -> DriverDescriptor {
        match self {
            Self::FabricLike(driver) => driver.descriptor(),
            Self::Quilt(driver) => driver.descriptor(),
            Self::LiteLoader(driver) => driver.descriptor(),
            Self::Rift(driver) => driver.descriptor(),
        }
    }

    pub async fn prepare<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        request: LoaderRequest,
    ) -> Result<ProfiledPrepared<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, request.descriptor) {
            (Self::FabricLike(driver), FABRIC_DRIVER | LEGACY_FABRIC_DRIVER | BABRIC_DRIVER) => {
                Ok(ProfiledPrepared::FabricLike(
                    driver
                        .prepare(
                            session,
                            instance,
                            request.game_version,
                            request.loader_version,
                        )
                        .await?,
                ))
            }
            (Self::Quilt(driver), QUILT_DRIVER) => Ok(ProfiledPrepared::Quilt(
                driver
                    .prepare(
                        session,
                        instance,
                        request.game_version,
                        request.loader_version,
                    )
                    .await?,
            )),
            (Self::LiteLoader(driver), LITELOADER_DRIVER) => Ok(ProfiledPrepared::LiteLoader(
                driver
                    .prepare(
                        session,
                        instance,
                        request.game_version,
                        request.loader_version,
                    )
                    .await?,
            )),
            (Self::Rift(driver), RIFT_DRIVER) => Ok(ProfiledPrepared::Rift(
                driver
                    .prepare(
                        session,
                        instance,
                        request.game_version,
                        request.loader_version,
                    )
                    .await?,
            )),
            (_, descriptor) => Err(Error::UnsupportedDriverFlow {
                id: descriptor.id.to_owned(),
            }),
        }
    }

    pub async fn load_prepared<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<ProfiledPrepared<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match self {
            Self::FabricLike(driver) => Ok(ProfiledPrepared::FabricLike(
                driver.load_prepared(instance).await?,
            )),
            Self::Quilt(driver) => Ok(ProfiledPrepared::Quilt(
                driver.load_prepared(instance).await?,
            )),
            Self::LiteLoader(driver) => Ok(ProfiledPrepared::LiteLoader(
                driver.load_prepared(instance).await?,
            )),
            Self::Rift(driver) => Ok(ProfiledPrepared::Rift(
                driver.load_prepared(instance).await?,
            )),
        }
    }

    pub async fn build_launch_command<A, L, VL>(
        &self,
        authorizer: A,
        prepared: &ProfiledPrepared<L, VL>,
        config: &VersionJsonLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, prepared) {
            (Self::FabricLike(driver), ProfiledPrepared::FabricLike(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            (Self::Quilt(driver), ProfiledPrepared::Quilt(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            (Self::LiteLoader(driver), ProfiledPrepared::LiteLoader(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            (Self::Rift(driver), ProfiledPrepared::Rift(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            _ => Err(Error::PreparedVariantMismatch {
                id: self.descriptor().id.to_owned(),
            }),
        }
    }
}

impl<L, VL> ProfiledPrepared<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn instance_root(&self) -> &Path {
        match self {
            Self::FabricLike(prepared) => prepared.resolved_version.version.path.as_path(),
            Self::Quilt(prepared) => prepared.resolved_version.version.path.as_path(),
            Self::LiteLoader(prepared) => prepared.resolved_version.version.path.as_path(),
            Self::Rift(prepared) => prepared.resolved_version.version.path.as_path(),
        }
    }

    pub fn required_java_major_version(&self) -> usize {
        match self {
            Self::FabricLike(prepared) => prepared.required_java_major_version(),
            Self::Quilt(prepared) => prepared.required_java_major_version(),
            Self::LiteLoader(prepared) => prepared.required_java_major_version(),
            Self::Rift(prepared) => prepared.required_java_major_version(),
        }
    }
}
