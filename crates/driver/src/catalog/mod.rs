mod collect;
mod loader_meta;
mod maven;
mod release;

use std::collections::HashMap;

use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::error::Result;

pub use collect::{
    collect_single_game_loader_releases, collect_single_game_releases, push_single_game_release,
};
pub use loader_meta::LoaderMetaCatalog;
pub use maven::MavenCatalog;
pub use release::LoaderRelease;

#[async_trait]
pub trait Catalog {
    type Release;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>>;
}
