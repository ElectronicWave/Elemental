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

pub struct DriverCatalog {
    driver: DriverDescriptor,
    game_versions: Arc<dyn GameVersionCatalog + Send + Sync>,
    loader_versions: Option<Arc<dyn LoaderCatalog + Send + Sync>>,
}

impl DriverCatalog {
    pub fn for_driver(driver: DriverDescriptor) -> Result<Self> {
        match driver {
            VANILLA_DRIVER => Ok(Self::game_only(driver, VanillaCatalog::with_defaults())),
            FABRIC_DRIVER => Self::with_loader(driver, FabricFlavor::Fabric),
            LEGACY_FABRIC_DRIVER => Self::with_loader(driver, FabricFlavor::LegacyFabric),
            BABRIC_DRIVER => Self::with_loader(driver, FabricFlavor::Babric),
            QUILT_DRIVER => Ok(Self::loader(driver, QuiltCatalog::with_defaults())),
            LITELOADER_DRIVER => Ok(Self::loader(driver, LiteLoaderCatalog::with_defaults())),
            RIFT_DRIVER => Ok(Self::loader(driver, RiftCatalog::with_defaults())),
            FORGE_DRIVER => Ok(Self::loader(driver, ForgeCatalog::with_defaults())),
            CLEANROOM_DRIVER => Ok(Self::loader(driver, CleanroomCatalog::with_defaults())),
            OUTLANDS_DRIVER => Ok(Self::loader(driver, OutlandsCatalog::with_defaults())),
            NEOFORGE_DRIVER => Ok(Self::loader(driver, NeoForgeCatalog::with_defaults())),
            _ => Err(Error::UnsupportedDriver {
                id: driver.id.to_owned(),
            }),
        }
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
