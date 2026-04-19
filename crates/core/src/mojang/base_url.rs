#[derive(Debug, Clone)]
pub struct MojangBaseUrl {
    pub launchermeta: String,
    pub pistonmeta: String,
    pub pistondata: String,
    pub resources: String,
    pub libraries: String,
}

impl Default for MojangBaseUrl {
    fn default() -> Self {
        Self {
            launchermeta: "launchermeta.mojang.com".to_owned(),
            pistonmeta: "piston-meta.mojang.com".to_owned(),
            resources: "resources.download.minecraft.net".to_owned(),
            libraries: "libraries.minecraft.net".to_owned(),
            pistondata: "piston-data.mojang.com".to_owned(),
        }
    }
}

impl MojangBaseUrl {
    pub fn get_object_url(&self, hash: impl AsRef<str>) -> String {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).expect("asset hash is too short");
        format!("https://{}/{prefix}/{hash}", self.resources)
    }
}
