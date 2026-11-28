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
use elemental_driver::{
    drivers::fabric::{
        FabricCatalog,
        source::{FabricEndpoints, FabricFlavor},
    },
    families::version_json::LoaderMetaSource,
};

trait LoaderCatalog: GameVersionCatalog + LoaderVersionCatalog {}

impl<T> LoaderCatalog for T where T: GameVersionCatalog + LoaderVersionCatalog {}

#[derive(Clone)]
pub struct DriverCatalog {
    driver: DriverDescriptor,
    game_versions: Arc<dyn GameVersionCatalog + Send + Sync>,
    loader_versions: Option<Arc<dyn LoaderCatalog + Send + Sync>>,
}

impl DriverCatalog {
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

    /// Builds a catalog that only supplies game versions.
    ///
    /// This is public so third-party drivers can expose their own catalog.
    pub fn game_only<G>(driver: DriverDescriptor, game_versions: G) -> Self
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
    pub(crate) fn with_loader(
        driver: DriverDescriptor,
        flavor: FabricFlavor,
        http_client: &elemental_core::http::SwappableClient,
    ) -> Result<Self> {
        let source = LoaderMetaSource::new(FabricEndpoints::for_flavor(flavor)?)
            .with_http_client(http_client.clone());
        Ok(Self::loader(driver, FabricCatalog::new(source)))
    }

    /// Builds a catalog that supplies both game and loader versions.
    ///
    /// This is public so third-party drivers can expose their own catalog.
    pub fn loader<C>(driver: DriverDescriptor, catalog: C) -> Self
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
