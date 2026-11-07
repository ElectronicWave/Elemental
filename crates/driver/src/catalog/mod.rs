mod loader_meta;
mod maven;
mod release;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::error::Result;

pub use loader_meta::LoaderMetaCatalog;
pub use maven::InstallerMavenCatalog;
pub use release::{GameVersion, LoaderRelease};

#[async_trait]
pub trait GameVersionCatalog {
    async fn game_versions(&self) -> Result<Vec<GameVersion>>;
}

#[async_trait]
pub trait LoaderVersionCatalog {
    async fn loader_versions(
        &self,
        game_version: &MinecraftVersionId,
    ) -> Result<Vec<LoaderRelease>>;
}
