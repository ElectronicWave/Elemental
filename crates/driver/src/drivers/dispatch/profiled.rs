use elemental_core::storage::Storage;
use elemental_infra::downloader::DownloadSession;

use crate::{
    drivers::{
        fabric::driver::FabricDriver, liteloader::driver::LiteLoaderDriver,
        quilt::driver::QuiltDriver, rift::driver::RiftDriver,
    },
    error::Result,
    families::version_json::{
        PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
};

use super::LoaderRequest;

pub enum ProfiledDriver {
    FabricLike(FabricDriver),
    Quilt(QuiltDriver),
    LiteLoader(LiteLoaderDriver),
    Rift(RiftDriver),
}

impl ProfiledDriver {
    pub async fn prepare<L, VL>(
        &self,
        session: &DownloadSession,
        instance: &Storage<VL, Storage<L>>,
        request: LoaderRequest,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
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
            ) => [FabricLike, Quilt, LiteLoader, Rift]
        )
    }

    pub async fn load_prepared<L, VL>(
        &self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<PreparedVersionJsonInstance<L, VL>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        dispatch_driver_await!(
            self,
            |driver| driver.load_prepared(instance) => [FabricLike, Quilt, LiteLoader, Rift]
        )
    }
}
