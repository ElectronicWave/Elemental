use crate::{
    error::unification::UnifiedResult,
    model::mojang::{VersionManifest, MojangBaseUrl, PistonMetaAssetIndexObjects, VersionData},
};
use std::io::Result;
#[derive(Debug)]
pub struct MojangService {
    pub baseurl: MojangBaseUrl,
}

impl Default for MojangService {
    fn default() -> Self {
        Self {
            baseurl: MojangBaseUrl::default(),
        }
    }
}

impl MojangService {
    pub fn new(baseurl: MojangBaseUrl) -> Self {
        Self { baseurl }
    }

    pub async fn get_version_manifest(&self) -> Result<VersionManifest> {
        // Use shortcut here because it wont call many times.
        reqwest::get(format!(
            "https://{}/mc/game/version_manifest_v2.json",
            self.baseurl.meta
        ))
        .await
        .to_stdio()?
        .json()
        .await
        .to_stdio()
    }

    pub async fn pistonmeta(&self, url: impl Into<String>) -> Result<VersionData> {
        reqwest::get(
            url.into()
                .replace("piston-meta.mojang.com", &self.baseurl.meta),
        )
        .await
        .to_stdio()?
        .json()
        .await
        .to_stdio()
    }

    pub async fn pistonmeta_v2(&self, url: impl Into<String>) {
        todo!()
    }

    pub async fn pistonmeta_assetindex_objects(
        &self,
        url: impl Into<String>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        reqwest::get(
            url.into()
                .replace("piston-meta.mojang.com", &self.baseurl.meta),
        )
        .await
        .to_stdio()?
        .json()
        .await
        .to_stdio()
    }
}

#[tokio::test]
async fn test_service() {
    let service = MojangService::default();
    let launch_meta = service.get_version_manifest().await.unwrap();
    let launch_meta_version_data = launch_meta.versions.first().unwrap();
    let pistonmeta = service
        .pistonmeta(launch_meta_version_data.url.clone())
        .await
        .unwrap();
    println!("{pistonmeta:?}")
}
