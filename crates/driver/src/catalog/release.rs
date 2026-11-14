use elemental_core::minecraft::MinecraftVersionId;

use crate::loader_version::LoaderVersionId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameVersion {
    pub id: MinecraftVersionId,
    pub channel: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LoaderRelease {
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
    pub channel: Option<String>,
    pub published_at: Option<String>,
    pub prerelease: Option<bool>,
}

impl LoaderRelease {
    pub fn new(game_version: MinecraftVersionId, loader_version: LoaderVersionId) -> Self {
        Self {
            game_version,
            loader_version,
            channel: None,
            published_at: None,
            prerelease: None,
        }
    }

    pub fn with_channel(mut self, channel: Option<String>) -> Self {
        self.channel = channel;
        self
    }

    pub fn with_published_at(mut self, published_at: Option<String>) -> Self {
        self.published_at = published_at;
        self
    }

    pub fn with_prerelease(mut self, prerelease: Option<bool>) -> Self {
        self.prerelease = prerelease;
        self
    }
}
