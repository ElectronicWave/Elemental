use std::path::Path;

use elemental_core::{
    auth::authorizer::Authorizer, launcher::command::LaunchCommand,
    runtime::distribution::Distribution, storage::Storage,
};
use elemental_infra::downloader::DownloadSession;

use crate::{
    descriptors::{CLEANROOM_DRIVER, FORGE_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER},
    driver::DriverDescriptor,
    drivers::{
        cleanroom::{CleanroomDriver, PreparedCleanroomVersion},
        forge::{ForgeDriver, PreparedForgeVersion},
        neoforge::{NeoForgeDriver, PreparedNeoForgeVersion},
        outlands::{OutlandsDriver, PreparedOutlandsVersion},
    },
    error::{Error, Result},
    families::version_json::{
        VersionJsonInstanceLayout, VersionJsonLaunchConfig, VersionJsonRootLayout,
    },
};

use super::LoaderRequest;

pub enum InstallerDriver {
    Forge(ForgeDriver),
    Cleanroom(CleanroomDriver),
    Outlands(OutlandsDriver),
    NeoForge(NeoForgeDriver),
}

pub enum InstallerPrepared<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    Forge(PreparedForgeVersion<L, VL>),
    Cleanroom(PreparedCleanroomVersion<L, VL>),
    Outlands(PreparedOutlandsVersion<L, VL>),
    NeoForge(PreparedNeoForgeVersion<L, VL>),
}

impl InstallerDriver {
    pub fn descriptor(&self) -> DriverDescriptor {
        match self {
            Self::Forge(driver) => driver.descriptor(),
            Self::Cleanroom(driver) => driver.descriptor(),
            Self::Outlands(driver) => driver.descriptor(),
            Self::NeoForge(driver) => driver.descriptor(),
        }
    }

    pub async fn prepare<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        request: LoaderRequest,
    ) -> Result<InstallerPrepared<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, request.descriptor) {
            (Self::Forge(driver), FORGE_DRIVER) => Ok(InstallerPrepared::Forge(
                driver
                    .prepare(
                        session,
                        instance,
                        request.game_version,
                        request.loader_version,
                    )
                    .await?,
            )),
            (Self::Cleanroom(driver), CLEANROOM_DRIVER) => Ok(InstallerPrepared::Cleanroom(
                driver
                    .prepare(
                        session,
                        instance,
                        request.game_version,
                        request.loader_version,
                    )
                    .await?,
            )),
            (Self::Outlands(driver), OUTLANDS_DRIVER) => Ok(InstallerPrepared::Outlands(
                driver
                    .prepare(
                        session,
                        instance,
                        request.game_version,
                        request.loader_version,
                    )
                    .await?,
            )),
            (Self::NeoForge(driver), NEOFORGE_DRIVER) => Ok(InstallerPrepared::NeoForge(
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
    ) -> Result<InstallerPrepared<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match self {
            Self::Forge(driver) => Ok(InstallerPrepared::Forge(
                driver.load_prepared(instance).await?,
            )),
            Self::Cleanroom(driver) => Ok(InstallerPrepared::Cleanroom(
                driver.load_prepared(instance).await?,
            )),
            Self::Outlands(driver) => Ok(InstallerPrepared::Outlands(
                driver.load_prepared(instance).await?,
            )),
            Self::NeoForge(driver) => Ok(InstallerPrepared::NeoForge(
                driver.load_prepared(instance).await?,
            )),
        }
    }

    pub async fn build_launch_command<A, L, VL>(
        &self,
        authorizer: A,
        prepared: &InstallerPrepared<L, VL>,
        config: &VersionJsonLaunchConfig,
    ) -> Result<(Distribution, LaunchCommand)>
    where
        A: Authorizer,
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        match (self, prepared) {
            (Self::Forge(driver), InstallerPrepared::Forge(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            (Self::Cleanroom(driver), InstallerPrepared::Cleanroom(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            (Self::Outlands(driver), InstallerPrepared::Outlands(prepared)) => {
                driver
                    .build_launch_command(authorizer, prepared, config)
                    .await
            }
            (Self::NeoForge(driver), InstallerPrepared::NeoForge(prepared)) => {
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

impl<L, VL> InstallerPrepared<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn instance_root(&self) -> &Path {
        match self {
            Self::Forge(prepared) => prepared.resolved_version.instance.path.as_path(),
            Self::Cleanroom(prepared) => prepared.resolved_version.instance.path.as_path(),
            Self::Outlands(prepared) => prepared.resolved_version.instance.path.as_path(),
            Self::NeoForge(prepared) => prepared.resolved_version.instance.path.as_path(),
        }
    }

    pub fn required_java_major_version(&self) -> usize {
        match self {
            Self::Forge(prepared) => prepared.required_java_major_version(),
            Self::Cleanroom(prepared) => prepared.required_java_major_version(),
            Self::Outlands(prepared) => prepared.required_java_major_version(),
            Self::NeoForge(prepared) => prepared.required_java_major_version(),
        }
    }
}
