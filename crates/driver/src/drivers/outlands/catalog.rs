use std::collections::HashMap;

use crate::error::Result;
use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;

use crate::catalog::{Catalog, collect_single_game_releases};
use crate::loader_version::LoaderVersionId;

use super::source::OutlandsSource;

pub const OUTLANDS_GAME_VERSION: &str = "1.12.2";
const OUTLANDS_RELEASE_DESCRIPTION: &str = "Outlands installer releases target Minecraft 1.12.2.";

#[derive(Debug, Clone)]
pub struct OutlandsCatalog {
    source: OutlandsSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OutlandsRelease {
    pub loader: LoaderVersionId,
    pub description: Option<String>,
}

impl Default for OutlandsCatalog {
    fn default() -> Self {
        Self::new(OutlandsSource::default())
    }
}

impl OutlandsCatalog {
    pub fn new(source: OutlandsSource) -> Self {
        Self { source }
    }

    pub fn source(&self) -> &OutlandsSource {
        &self.source
    }
}

#[async_trait]
impl Catalog for OutlandsCatalog {
    type Release = OutlandsRelease;

    async fn releases(&self) -> Result<HashMap<MinecraftVersionId, Vec<Self::Release>>> {
        let metadata = self.source.maven_metadata().await?;
        Ok(collect_single_game_releases(
            metadata.versioning.versions.version,
            |version| {
                Some((
                    MinecraftVersionId::from(OUTLANDS_GAME_VERSION),
                    OutlandsRelease {
                        loader: LoaderVersionId::from(version),
                        description: Some(OUTLANDS_RELEASE_DESCRIPTION.to_owned()),
                    },
                ))
            },
        ))
    }
}
