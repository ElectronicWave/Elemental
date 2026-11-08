use std::sync::Arc;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_driver::{
    catalog::{
        GameVersion, GameVersionCatalog, LoaderMetaCatalog, LoaderRelease, LoaderVersionCatalog,
    },
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::DriverDescriptor,
    drivers::{
        cleanroom::CleanroomCatalog,
        fabric::source::{FabricEndpoints, FabricFlavor},
        forge::ForgeCatalog,
        liteloader::catalog::LiteLoaderCatalog,
        neoforge::NeoForgeCatalog,
        outlands::OutlandsCatalog,
        quilt::catalog::QuiltCatalog,
        rift::catalog::RiftCatalog,
        vanilla::catalog::VanillaCatalog,
    },
    families::version_json::LoaderMetaSource,
};

use crate::error::{Error, Result};

trait LoaderCatalog: GameVersionCatalog + LoaderVersionCatalog {}

impl<T> LoaderCatalog for T where T: GameVersionCatalog + LoaderVersionCatalog {}

type CatalogFactory = fn(DriverDescriptor) -> Result<DriverCatalog>;

struct DriverRegistration {
    descriptor: DriverDescriptor,
    factory: CatalogFactory,
}

const DRIVER_REGISTRATIONS: [DriverRegistration; 11] = [
    DriverRegistration {
        descriptor: VANILLA_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::game_only(
                driver,
                VanillaCatalog::with_defaults(),
            ))
        },
    },
    DriverRegistration {
        descriptor: FABRIC_DRIVER,
        factory: |driver| DriverCatalog::with_loader(driver, FabricFlavor::Fabric),
    },
    DriverRegistration {
        descriptor: LEGACY_FABRIC_DRIVER,
        factory: |driver| DriverCatalog::with_loader(driver, FabricFlavor::LegacyFabric),
    },
    DriverRegistration {
        descriptor: BABRIC_DRIVER,
        factory: |driver| DriverCatalog::with_loader(driver, FabricFlavor::Babric),
    },
    DriverRegistration {
        descriptor: QUILT_DRIVER,
        factory: |driver| Ok(DriverCatalog::loader(driver, QuiltCatalog::with_defaults())),
    },
    DriverRegistration {
        descriptor: LITELOADER_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                LiteLoaderCatalog::with_defaults(),
            ))
        },
    },
    DriverRegistration {
        descriptor: RIFT_DRIVER,
        factory: |driver| Ok(DriverCatalog::loader(driver, RiftCatalog::with_defaults())),
    },
    DriverRegistration {
        descriptor: FORGE_DRIVER,
        factory: |driver| Ok(DriverCatalog::loader(driver, ForgeCatalog::with_defaults())),
    },
    DriverRegistration {
        descriptor: CLEANROOM_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                CleanroomCatalog::with_defaults(),
            ))
        },
    },
    DriverRegistration {
        descriptor: OUTLANDS_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                OutlandsCatalog::with_defaults(),
            ))
        },
    },
    DriverRegistration {
        descriptor: NEOFORGE_DRIVER,
        factory: |driver| {
            Ok(DriverCatalog::loader(
                driver,
                NeoForgeCatalog::with_defaults(),
            ))
        },
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
