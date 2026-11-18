#![cfg_attr(not(feature = "driver-all"), allow(dead_code, unreachable_code))]

use std::sync::Arc;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::{GameVersion, GameVersionCatalog, LoaderRelease, LoaderVersionCatalog},
    driver::DriverDescriptor,
};

use crate::error::{Error, Result};

#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use elemental_driver::catalog::LoaderMetaCatalog;

#[cfg(feature = "driver-babric")]
use elemental_driver::descriptors::BABRIC_DRIVER;
#[cfg(feature = "driver-fabric")]
use elemental_driver::descriptors::FABRIC_DRIVER;
#[cfg(feature = "driver-legacyfabric")]
use elemental_driver::descriptors::LEGACY_FABRIC_DRIVER;
#[cfg(feature = "driver-cleanroom")]
use elemental_driver::{descriptors::CLEANROOM_DRIVER, drivers::cleanroom::CleanroomCatalog};
#[cfg(feature = "driver-forge")]
use elemental_driver::{descriptors::FORGE_DRIVER, drivers::forge::ForgeCatalog};
#[cfg(feature = "driver-liteloader")]
use elemental_driver::{
    descriptors::LITELOADER_DRIVER, drivers::liteloader::catalog::LiteLoaderCatalog,
};
#[cfg(feature = "driver-neoforge")]
use elemental_driver::{descriptors::NEOFORGE_DRIVER, drivers::neoforge::NeoForgeCatalog};
#[cfg(feature = "driver-outlands")]
use elemental_driver::{descriptors::OUTLANDS_DRIVER, drivers::outlands::OutlandsCatalog};
#[cfg(feature = "driver-quilt")]
use elemental_driver::{descriptors::QUILT_DRIVER, drivers::quilt::catalog::QuiltCatalog};
#[cfg(feature = "driver-rift")]
use elemental_driver::{descriptors::RIFT_DRIVER, drivers::rift::catalog::RiftCatalog};
#[cfg(feature = "driver-vanilla")]
use elemental_driver::{descriptors::VANILLA_DRIVER, drivers::vanilla::catalog::VanillaCatalog};
#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use elemental_driver::{
    drivers::fabric::source::{FabricEndpoints, FabricFlavor},
    families::version_json::LoaderMetaSource,
};

trait LoaderCatalog: GameVersionCatalog + LoaderVersionCatalog {}

impl<T> LoaderCatalog for T where T: GameVersionCatalog + LoaderVersionCatalog {}

type CatalogFactory = fn(DriverDescriptor) -> Result<DriverCatalog>;

struct DriverRegistration {
    descriptor: DriverDescriptor,
    factory: CatalogFactory,
}

/// Drivers in the order surfaced by [`DriverCatalog::supported_drivers`].
///
/// Actively maintained drivers come first: mainstream families, then the
/// maintained legacy families, then the two maintained 1.12.2 drivers.
/// Outlands precedes Cleanroom because Outlands is the actively maintained
/// LGPL fork, while Cleanroom uses a source-available license. Archived or
/// abandoned drivers stay last.
const DRIVER_REGISTRATIONS: &[DriverRegistration] = &[
    #[cfg(feature = "driver-vanilla")]
    DriverRegistration {
        descriptor: VANILLA_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::game_only(
                driver,
                VanillaCatalog::with_defaults(),
            ))
        },
    },
    #[cfg(feature = "driver-fabric")]
    DriverRegistration {
        descriptor: FABRIC_DRIVER,
        factory: |driver| DriverCatalog::with_loader(driver, FabricFlavor::Fabric),
    },
    #[cfg(feature = "driver-neoforge")]
    DriverRegistration {
        descriptor: NEOFORGE_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                NeoForgeCatalog::with_defaults(),
            ))
        },
    },
    #[cfg(feature = "driver-forge")]
    DriverRegistration {
        descriptor: FORGE_DRIVER,
        factory: |driver| Ok(DriverCatalog::loader(driver, ForgeCatalog::with_defaults())),
    },
    #[cfg(feature = "driver-quilt")]
    DriverRegistration {
        descriptor: QUILT_DRIVER,
        factory: |driver| Ok(DriverCatalog::loader(driver, QuiltCatalog::with_defaults())),
    },
    #[cfg(feature = "driver-legacyfabric")]
    DriverRegistration {
        descriptor: LEGACY_FABRIC_DRIVER,
        factory: |driver| DriverCatalog::with_loader(driver, FabricFlavor::LegacyFabric),
    },
    #[cfg(feature = "driver-babric")]
    DriverRegistration {
        descriptor: BABRIC_DRIVER,
        factory: |driver| DriverCatalog::with_loader(driver, FabricFlavor::Babric),
    },
    #[cfg(feature = "driver-outlands")]
    DriverRegistration {
        descriptor: OUTLANDS_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                OutlandsCatalog::with_defaults(),
            ))
        },
    },
    #[cfg(feature = "driver-cleanroom")]
    DriverRegistration {
        descriptor: CLEANROOM_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                CleanroomCatalog::with_defaults(),
            ))
        },
    },
    #[cfg(feature = "driver-liteloader")]
    DriverRegistration {
        descriptor: LITELOADER_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                LiteLoaderCatalog::with_defaults(),
            ))
        },
    },
    #[cfg(feature = "driver-rift")]
    DriverRegistration {
        descriptor: RIFT_DRIVER,
        factory: |driver| Ok(DriverCatalog::loader(driver, RiftCatalog::with_defaults())),
    },
];

pub struct DriverCatalog {
    driver: DriverDescriptor,
    game_versions: Arc<dyn GameVersionCatalog + Send + Sync>,
    loader_versions: Option<Arc<dyn LoaderCatalog + Send + Sync>>,
}

impl DriverCatalog {
    pub fn supported_drivers() -> impl Iterator<Item = DriverDescriptor> {
        DRIVER_REGISTRATIONS
            .iter()
            .map(|registration| registration.descriptor)
    }

    pub fn for_driver(driver: DriverDescriptor) -> Result<Self> {
        let registration = DRIVER_REGISTRATIONS
            .iter()
            .find(|registration| registration.descriptor == driver)
            .ok_or_else(|| Error::UnsupportedDriver {
                id: driver.id.to_owned(),
            })?;

        (registration.factory)(driver)
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
        Ok(Self::loader(driver, LoaderMetaCatalog::new(source)))
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
