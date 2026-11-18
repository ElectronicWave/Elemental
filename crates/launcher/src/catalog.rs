#![cfg_attr(not(feature = "driver-all"), allow(dead_code, unreachable_code))]
#![allow(unreachable_patterns)]

use std::sync::Arc;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog},
    descriptors::{
        ALL_DRIVERS, BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER,
        LEGACY_FABRIC_DRIVER, LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER,
        RIFT_DRIVER, VANILLA_DRIVER,
    },
    driver::DriverDescriptor,
};

use crate::error::{Error, Result};

#[cfg(feature = "driver-cleanroom")]
use elemental_driver::drivers::cleanroom::CleanroomCatalog;
#[cfg(feature = "driver-forge")]
use elemental_driver::drivers::forge::ForgeCatalog;
#[cfg(feature = "driver-liteloader")]
use elemental_driver::drivers::liteloader::catalog::LiteLoaderCatalog;
#[cfg(feature = "driver-neoforge")]
use elemental_driver::drivers::neoforge::NeoForgeCatalog;
#[cfg(feature = "driver-outlands")]
use elemental_driver::drivers::outlands::OutlandsCatalog;
#[cfg(feature = "driver-quilt")]
use elemental_driver::drivers::quilt::QuiltCatalog;
#[cfg(feature = "driver-rift")]
use elemental_driver::drivers::rift::catalog::RiftCatalog;
#[cfg(feature = "driver-vanilla")]
use elemental_driver::drivers::vanilla::catalog::VanillaCatalog;
#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use elemental_driver::{
    drivers::fabric::{
        FabricCatalog,
        source::{FabricEndpoints, FabricFlavor},
    },
    families::version_json::LoaderMetaSource,
};

trait LoaderCatalog: GameVersionCatalog + LoaderVersionCatalog {}

impl<T> LoaderCatalog for T where T: GameVersionCatalog + LoaderVersionCatalog {}

pub struct DriverCatalog {
    driver: DriverDescriptor,
    game_versions: Arc<dyn GameVersionCatalog + Send + Sync>,
    loader_versions: Option<Arc<dyn LoaderCatalog + Send + Sync>>,
}

impl DriverCatalog {
    pub fn supported_drivers() -> impl Iterator<Item = DriverDescriptor> {
        let mut descriptors = ALL_DRIVERS.to_vec();
        descriptors.sort_by_key(|descriptor| catalog_order(*descriptor));
        descriptors.into_iter()
    }

    pub fn for_driver(driver: DriverDescriptor) -> Result<Self> {
        catalog_for(driver)
    }

    pub async fn game_versions(&self) -> Result<Vec<GameVersion>> {
        Ok(self.game_versions.game_versions().await?)
    }

    pub fn driver(&self) -> DriverDescriptor {
        self.driver
    }

    pub async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>> {
        let Some(catalog) = self.loader_versions.as_ref() else {
            return Err(Error::LoaderVersionsUnsupported {
                id: self.driver.id.to_owned(),
            });
        };

        Ok(catalog.loader_versions(game_version).await?)
    }

    fn game_only<G>(driver: DriverDescriptor, game_versions: G) -> Self
    where
        G: GameVersionCatalog + Send + Sync + 'static,
    {
        Self {
            driver,
            game_versions: Arc::new(game_versions),
            loader_versions: None,
        }
    }

    #[cfg(any(
        feature = "driver-fabric",
        feature = "driver-legacyfabric",
        feature = "driver-babric"
    ))]
    fn with_loader(driver: DriverDescriptor, flavor: FabricFlavor) -> Result<Self> {
        let source = LoaderMetaSource::new(FabricEndpoints::for_flavor(flavor)?);
        Ok(Self::loader(driver, FabricCatalog::new(source)))
    }

    fn loader<C>(driver: DriverDescriptor, catalog: C) -> Self
    where
        C: GameVersionCatalog + LoaderVersionCatalog + Send + Sync + 'static,
    {
        let loader_versions: Arc<dyn LoaderCatalog + Send + Sync> = Arc::new(catalog);
        let game_versions: Arc<dyn GameVersionCatalog + Send + Sync> = loader_versions.clone();
        Self {
            driver,
            game_versions,
            loader_versions: Some(loader_versions),
        }
    }
}

fn catalog_order(descriptor: DriverDescriptor) -> u8 {
    match descriptor {
        VANILLA_DRIVER => 0,
        FABRIC_DRIVER => 1,
        NEOFORGE_DRIVER => 2,
        FORGE_DRIVER => 3,
        QUILT_DRIVER => 4,
        LEGACY_FABRIC_DRIVER => 5,
        BABRIC_DRIVER => 6,
        OUTLANDS_DRIVER => 7,
        CLEANROOM_DRIVER => 8,
        LITELOADER_DRIVER => 9,
        RIFT_DRIVER => 10,
        _ => u8::MAX,
    }
}

fn catalog_for(driver: DriverDescriptor) -> Result<DriverCatalog> {
    Ok(match driver {
        #[cfg(feature = "driver-vanilla")]
        VANILLA_DRIVER => DriverCatalog::game_only(driver, VanillaCatalog::with_defaults()),
        #[cfg(feature = "driver-fabric")]
        FABRIC_DRIVER => DriverCatalog::with_loader(driver, FabricFlavor::Fabric)?,
        #[cfg(feature = "driver-legacyfabric")]
        LEGACY_FABRIC_DRIVER => DriverCatalog::with_loader(driver, FabricFlavor::LegacyFabric)?,
        #[cfg(feature = "driver-babric")]
        BABRIC_DRIVER => DriverCatalog::with_loader(driver, FabricFlavor::Babric)?,
        #[cfg(feature = "driver-quilt")]
        QUILT_DRIVER => DriverCatalog::loader(driver, QuiltCatalog::with_defaults()),
        #[cfg(feature = "driver-liteloader")]
        LITELOADER_DRIVER => DriverCatalog::loader(driver, LiteLoaderCatalog::with_defaults()),
        #[cfg(feature = "driver-rift")]
        RIFT_DRIVER => DriverCatalog::loader(driver, RiftCatalog::with_defaults()),
        #[cfg(feature = "driver-forge")]
        FORGE_DRIVER => DriverCatalog::loader(driver, ForgeCatalog::with_defaults()),
        #[cfg(feature = "driver-cleanroom")]
        CLEANROOM_DRIVER => DriverCatalog::loader(driver, CleanroomCatalog::with_defaults()),
        #[cfg(feature = "driver-outlands")]
        OUTLANDS_DRIVER => DriverCatalog::loader(driver, OutlandsCatalog::with_defaults()),
        #[cfg(feature = "driver-neoforge")]
        NEOFORGE_DRIVER => DriverCatalog::loader(driver, NeoForgeCatalog::with_defaults()),
        _ => {
            return Err(Error::UnsupportedDriver {
                id: driver.id.to_owned(),
            });
        }
    })
}
