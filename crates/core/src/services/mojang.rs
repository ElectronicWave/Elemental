use crate::models::mojang::{
    LaunchMetaData, MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData,
};
use anyhow::Result;
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

    pub async fn launchmeta(&self) -> Result<LaunchMetaData> {
        // Use shortcut here because it wont call many times.
        Ok(reqwest::get(format!(
            "https://{}/mc/game/version_manifest_v2.json",
            self.baseurl.launchermeta
        ))
        .await?
        .json()
        .await?)
    }

    pub async fn launchmeta_v2(&self) {
        todo!()
    }

    pub async fn pistonmeta(&self, url: impl Into<String>) -> Result<PistonMetaData> {
        Ok(reqwest::get(
            url.into()
                .replace("piston-meta.mojang.com", &self.baseurl.pistonmeta),
        )
        .await?
        .json()
        .await?)
    }

    pub async fn pistonmeta_v2(&self, url: impl Into<String>) {
        todo!()
    }

    pub async fn pistonmeta_assetindex_objects(
        &self,
        url: impl Into<String>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        Ok(reqwest::get(
            url.into()
                .replace("piston-meta.mojang.com", &self.baseurl.pistonmeta),
        )
        .await?
        .json()
        .await?)
    }
}

#[tokio::test]
async fn test_service() {
    let service = MojangService::default();
    let launch_meta = service.launchmeta().await.unwrap();
    let launch_meta_version_data = launch_meta.versions.first().unwrap();
    let pistonmeta = service
        .pistonmeta(launch_meta_version_data.url.clone())
        .await
        .unwrap();
    println!("{pistonmeta:?}")
}
