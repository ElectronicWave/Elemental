use crate::model::mojang::{
    LaunchMetaData, MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData,
};
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

    pub async fn launchmeta(&self) -> Result<LaunchMetaData, reqwest::Error> {
        let url: String;

        if self.baseurl.launchermeta_https {
            url = format!(
                "https://{}/mc/game/version_manifest.json",
                self.baseurl.launchermeta
            );
        } else {
            url = format!(
                "http://{}/mc/game/version_manifest.json",
                self.baseurl.launchermeta
            );
        }
        // Use shortcut here because it wont call many times.
        reqwest::get(url).await?.json().await
    }

    pub async fn pistonmeta(
        &self,
        url: impl Into<String>,
    ) -> Result<PistonMetaData, reqwest::Error> {
        reqwest::get(
            url.into()
                .replace("piston-meta.mojang.com", &self.baseurl.pistonmeta),
        )
        .await?
        .json()
        .await
    }

    pub async fn pistonmeta_assetindex_objects(
        &self,
        url: impl Into<String>,
    ) -> Result<PistonMetaAssetIndexObjects, reqwest::Error> {
        reqwest::get(
            url.into()
                .replace("piston-meta.mojang.com", &self.baseurl.pistonmeta),
        )
        .await?
        .json()
        .await
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
