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
    pub fn version_manifest_url(&self) -> String {
        format!(
            "https://{}/mc/game/version_manifest_v2.json",
            self.launchermeta
        )
    }

    pub fn rewrite_pistonmeta_url(&self, url: impl Into<String>) -> String {
        url.into()
            .replace("piston-meta.mojang.com", &self.pistonmeta)
    }

    pub fn rewrite_pistondata_url(&self, url: impl Into<String>) -> String {
        url.into()
            .replace("piston-data.mojang.com", &self.pistondata)
    }

    pub fn rewrite_library_url(&self, url: impl Into<String>) -> String {
        url.into()
            .replace("libraries.minecraft.net", &self.libraries)
    }

    pub fn get_object_url(&self, hash: impl AsRef<str>) -> String {
        let hash = hash.as_ref();
        let prefix = hash.get(0..2).expect("asset hash is too short");
        format!("https://{}/{prefix}/{hash}", self.resources)
    }
}
