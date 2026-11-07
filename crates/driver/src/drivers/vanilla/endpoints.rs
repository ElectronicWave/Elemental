use crate::{
    error::{Error, Result},
    families::version_json::VersionJsonRemoteResolver,
    url::{Origin, OriginPolicy},
};

mod mirror;

const LAUNCHERMETA_ORIGIN: &str = "https://launchermeta.mojang.com";
const PISTONMETA_ORIGIN: &str = "https://piston-meta.mojang.com";
const PISTONDATA_ORIGIN: &str = "https://piston-data.mojang.com";
const RESOURCES_ORIGIN: &str = "https://resources.download.minecraft.net";
const LIBRARIES_ORIGIN: &str = "https://libraries.minecraft.net";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VanillaOrigin {
    LauncherMeta,
    PistonMeta,
    PistonData,
    Resources,
    Libraries,
}

#[derive(Debug, Clone)]
pub struct VanillaEndpoints {
    origin_policy: OriginPolicy<VanillaOrigin>,
}

impl Default for VanillaEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for VanillaOrigin {
    fn canonical(self) -> &'static str {
        match self {
            VanillaOrigin::LauncherMeta => LAUNCHERMETA_ORIGIN,
            VanillaOrigin::PistonMeta => PISTONMETA_ORIGIN,
            VanillaOrigin::PistonData => PISTONDATA_ORIGIN,
            VanillaOrigin::Resources => RESOURCES_ORIGIN,
            VanillaOrigin::Libraries => LIBRARIES_ORIGIN,
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[VanillaOrigin] = &[
            VanillaOrigin::LauncherMeta,
            VanillaOrigin::PistonMeta,
            VanillaOrigin::PistonData,
            VanillaOrigin::Resources,
            VanillaOrigin::Libraries,
        ];
        ALL
    }
}

impl VanillaEndpoints {
    pub fn new(origin_policy: OriginPolicy<VanillaOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn origin_policy(&self) -> &OriginPolicy<VanillaOrigin> {
        &self.origin_policy
    }

    pub fn version_manifest_url(&self) -> Result<String> {
        self.origin_policy.resolve(
            VanillaOrigin::LauncherMeta,
            "/mc/game/version_manifest_v2.json",
        )
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_origin_url(raw_url)
    }

    pub fn object_url(&self, hash: impl AsRef<str>) -> Result<String> {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).ok_or_else(|| Error::AssetHashTooShort {
            hash: hash.to_owned(),
        })?;
        self.origin_policy
            .resolve(VanillaOrigin::Resources, &format!("{prefix}/{hash}"))
    }
}

impl VersionJsonRemoteResolver for VanillaEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.rewrite_upstream(raw_url)
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.object_url(hash)
    }
}
