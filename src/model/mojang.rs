use serde::Deserialize;

pub struct MojangBaseUrl {
    pub launchermeta: String,
    pub launchermeta_https: bool,
    pub pistonmeta: String,
}

impl Default for MojangBaseUrl {
    fn default() -> Self {
        Self {
            launchermeta: "launchermeta.mojang.com".to_owned(),
            launchermeta_https: false,
            pistonmeta: "piston-meta.mojang.com".to_owned(),
        }
    }
}

/// http://launchermeta.mojang.com/mc/game/version_manifest.json
#[derive(Debug, Deserialize)]
pub struct LaunchMetaData {
    pub latest: LaunchMetaLatestData,
    pub versions: Vec<LaunchMetaVersionData>,
}

#[derive(Debug, Deserialize)]
pub struct LaunchMetaLatestData {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Deserialize)]
pub struct LaunchMetaVersionData {
    pub id: String,
    #[serde(rename = "type")]
    pub typo: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// https://piston-meta.mojang.com/v1/packages/<->/<->.json
pub struct PistonMetaData {
    //TODO
}
