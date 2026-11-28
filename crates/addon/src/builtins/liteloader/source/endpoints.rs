use elemental_core::url::{Origin, OriginPolicy};

use crate::error::Result;

pub(super) const KNOWN_LITELOADER_ORIGINS: &[&str] = &[
    "https://dl.liteloader.com/versions",
    "https://repo.mumfrey.com/content/repositories/snapshots",
    "https://repo.spongepowered.org/maven",
    "https://libraries.minecraft.net",
    "https://repo.maven.apache.org/maven2",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteLoaderOrigin {
    Downloads,
    Snapshots,
    SpongeMaven,
    MojangLibraries,
    MavenCentral,
}

#[derive(Debug, Clone)]
pub struct LiteLoaderEndpoints {
    pub(super) origin_policy: OriginPolicy,
}

impl Default for LiteLoaderEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for LiteLoaderOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::Downloads => "https://dl.liteloader.com/versions",
            Self::Snapshots => "https://repo.mumfrey.com/content/repositories/snapshots",
            Self::SpongeMaven => "https://repo.spongepowered.org/maven",
            Self::MojangLibraries => "https://libraries.minecraft.net",
            Self::MavenCentral => "https://repo.maven.apache.org/maven2",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[LiteLoaderOrigin] = &[
            LiteLoaderOrigin::Downloads,
            LiteLoaderOrigin::Snapshots,
            LiteLoaderOrigin::SpongeMaven,
            LiteLoaderOrigin::MojangLibraries,
            LiteLoaderOrigin::MavenCentral,
        ];
        ALL
    }
}

impl AsRef<str> for LiteLoaderOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

impl LiteLoaderEndpoints {
    pub fn new(origin_policy: OriginPolicy) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn versions_manifest_url(&self) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(LiteLoaderOrigin::Downloads, "versions.json")?)
    }

    pub fn downloads_url(&self) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(LiteLoaderOrigin::Downloads, "")?)
    }

    pub fn snapshots_url(&self) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(LiteLoaderOrigin::Snapshots, "")?)
    }

    pub fn sponge_maven_url(&self) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(LiteLoaderOrigin::SpongeMaven, "")?)
    }

    pub fn mojang_libraries_url(&self) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(LiteLoaderOrigin::MojangLibraries, "")?)
    }

    pub fn maven_central_url(&self) -> Result<String> {
        Ok(self
            .origin_policy
            .resolve(LiteLoaderOrigin::MavenCentral, "")?)
    }
}
