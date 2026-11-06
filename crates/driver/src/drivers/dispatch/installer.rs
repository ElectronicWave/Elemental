use elemental_core::storage::Storage;
use elemental_infra::downloader::DownloadSession;

use crate::{
    drivers::{
        cleanroom::CleanroomDriver, forge::ForgeDriver, neoforge::NeoForgeDriver,
        outlands::OutlandsDriver,
    },
    error::Result,
    families::{
        installer::PreparedInstallerVersion,
        version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    },
};

use super::LoaderRequest;

pub enum InstallerDriver {
    Forge(ForgeDriver),
    Cleanroom(CleanroomDriver),
    Outlands(OutlandsDriver),
    NeoForge(NeoForgeDriver),
}

impl InstallerDriver {
    pub async fn prepare<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        request: LoaderRequest,
    ) -> Result<PreparedInstallerVersion<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        dispatch_driver_await!(
            self,
            |driver| driver.prepare(
                session,
                instance,
                request.game_version,
                request.loader_version,
            ) => [Forge, Cleanroom, Outlands, NeoForge]
        )
    }

    pub async fn load_prepared<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedInstallerVersion<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        dispatch_driver_await!(
            self,
            |driver| driver.load_prepared(instance) => [Forge, Cleanroom, Outlands, NeoForge]
        )
    }
}
