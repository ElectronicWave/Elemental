use std::time::Duration;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

use crate::{
    drivers::version_json::{LaunchMetaData, PistonMetaAssetIndexObjects, PistonMetaData},
    url::{ReplacePrefixRule, UrlMapper},
};

const LAUNCHERMETA_ORIGIN: &str = "https://launchermeta.mojang.com";
const PISTONMETA_ORIGIN: &str = "https://piston-meta.mojang.com";
const PISTONDATA_ORIGIN: &str = "https://piston-data.mojang.com";
const RESOURCES_ORIGIN: &str = "https://resources.download.minecraft.net";
const LIBRARIES_ORIGIN: &str = "https://libraries.minecraft.net";

#[derive(Debug, Clone)]
pub struct VanillaEndpoints {
    mapper: UrlMapper,
}

#[derive(Debug, Clone)]
pub struct VanillaSource {
    client: reqwest::Client,
    endpoints: VanillaEndpoints,
}

impl Default for VanillaEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl VanillaEndpoints {
    pub fn new(mapper: UrlMapper) -> Self {
        Self { mapper }
    }

    pub fn official() -> Self {
        Self::new(UrlMapper::default())
    }

    pub fn mirror(
        launchermeta_origin: String,
        pistonmeta_origin: String,
        pistondata_origin: String,
        resources_origin: String,
        libraries_origin: String,
    ) -> Self {
        let mapper = UrlMapper::default()
            .add_rule(ReplacePrefixRule::new(
                LAUNCHERMETA_ORIGIN.to_owned(),
                launchermeta_origin,
            ))
            .add_rule(ReplacePrefixRule::new(
                PISTONMETA_ORIGIN.to_owned(),
                pistonmeta_origin,
            ))
            .add_rule(ReplacePrefixRule::new(
                PISTONDATA_ORIGIN.to_owned(),
                pistondata_origin,
            ))
            .add_rule(ReplacePrefixRule::new(
                RESOURCES_ORIGIN.to_owned(),
                resources_origin,
            ))
            .add_rule(ReplacePrefixRule::new(
                LIBRARIES_ORIGIN.to_owned(),
                libraries_origin,
            ));
        Self::new(mapper)
    }

    pub fn mapper(&self) -> &UrlMapper {
        &self.mapper
    }

    pub fn version_manifest_url(&self) -> Result<String> {
        self.mapper.rewrite(format!(
            "{LAUNCHERMETA_ORIGIN}/mc/game/version_manifest_v2.json"
        ))
    }

    pub fn rewrite(&self, url: impl AsRef<str>) -> Result<String> {
        self.mapper.rewrite(url)
    }

    pub fn object_url(&self, hash: impl AsRef<str>) -> Result<String> {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).context("asset hash is too short")?;
        self.mapper
            .rewrite(format!("{RESOURCES_ORIGIN}/{prefix}/{hash}"))
    }
}

impl Default for VanillaSource {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(format!("Elemental/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("build vanilla source client failed"),
            endpoints: VanillaEndpoints::default(),
        }
    }
}

impl VanillaSource {
    pub fn new(endpoints: VanillaEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn with_client(endpoints: VanillaEndpoints, client: reqwest::Client) -> Self {
        Self { client, endpoints }
    }

    pub fn endpoints(&self) -> &VanillaEndpoints {
        &self.endpoints
    }

    pub async fn launch_meta(&self) -> Result<LaunchMetaData> {
        let url = self.endpoints.version_manifest_url()?;
        self.fetch_json(url.as_str()).await
    }

    pub async fn piston_meta(&self, url: impl AsRef<str>) -> Result<PistonMetaData> {
        let url = self.endpoints.rewrite(url)?;
        self.fetch_json(url.as_str()).await
    }

    pub async fn asset_index_objects(
        &self,
        url: impl AsRef<str>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let url = self.endpoints.rewrite(url)?;
        self.fetch_json(url.as_str()).await
    }

    async fn fetch_json<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.client
            .get(url)
            .send()
            .await
            .with_context(|| format!("request vanilla source resource failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("vanilla source returned error status: {url}"))?
            .json::<T>()
            .await
            .with_context(|| format!("decode vanilla source resource failed: {url}"))
    }
}
