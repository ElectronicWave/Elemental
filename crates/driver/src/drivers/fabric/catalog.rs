use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::catalog::{
    Catalog, GameVersions, Release, ReleaseInfo, collect_single_game_loader_releases,
    single_game_release_info,
};
use crate::loader_version::LoaderVersionId;

use super::source::{FabricEndpointOverrides, FabricEndpoints, FabricFlavor, FabricSource};

pub struct FabricCatalog {
    source: FabricSource,
}

pub struct FabricRelease {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
    pub description: Option<String>,
}

#[async_trait]
impl Release for FabricRelease {
    async fn info(&self) -> ReleaseInfo {
        single_game_release_info(
            self.loader_version.to_string(),
            self.game_version.clone(),
            self.description.clone(),
        )
    }
}

impl FabricCatalog {
    pub fn new(source: FabricSource) -> Self {
        Self { source }
    }

    pub fn with_defaults() -> Self {
        Self::new(FabricSource::default())
    }

    pub fn for_flavor(flavor: FabricFlavor) -> Result<Self> {
        Ok(Self::new(FabricSource::new(FabricEndpoints::for_flavor(
            flavor,
        )?)))
    }

    pub fn with_overrides(overrides: FabricEndpointOverrides) -> Result<Self> {
        Ok(Self::new(FabricSource::new(
            FabricEndpoints::with_overrides(overrides)?,
        )))
    }
}

#[async_trait]
impl Catalog for FabricCatalog {
    type Release = FabricRelease;

    async fn releases(&self) -> Result<HashMap<GameVersions, Vec<Self::Release>>> {
        let game_versions = self
            .source
            .game_versions()
            .await?
            .into_iter()
            .map(|game_version| MinecraftVersionId::from(game_version.version))
            .collect::<Vec<MinecraftVersionId>>();

        collect_single_game_loader_releases(
            game_versions,
            |game_version| async move { self.source.loader_versions(game_version.as_str()).await },
            |game_version, loader| FabricRelease {
                game_version: game_version.clone(),
                loader_version: LoaderVersionId::from(loader.loader.version),
                description: Some(if loader.loader.stable {
                    "Stable".to_owned()
                } else {
                    "Unstable".to_owned()
                }),
            },
        )
        .await
    }
}
