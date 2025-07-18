use crate::curseforge::serialize::{Desc, GetMod, GetModFile, GetModFiles, SearchMods};
use crate::{Discover, UrlBuilder};

pub mod serialize;

#[derive(Debug)]
pub struct Curse {
    base_url: String,
    api_key: String,
    special_mode: bool,
}

/* Minecraft Game ID = 432
 * Category ID:
    World - 17
    Mods - 6
    Modpacks - 4471
    Resource Packs - 12
    Shaders - 6552
 *
 */

impl Curse {
    pub fn new_with_default_key(special_mode: bool) -> Self {
        Self {
            base_url: "https://api.curseforge.com/v1/".to_string(),
            api_key: "$2a$10$m.XJaH.ysAKZ3VzWIfnlP.RXkN54zQSYIUxBM8/H5riHQ2cUx3koy".to_string(), // Default API key
            special_mode,
        }
    }

    pub fn new(api_key: String, special_mode: bool) -> Self {
        Self {
            base_url: "https://api.curseforge.com/v1/".to_string(),
            api_key,
            special_mode,
        }
    }

    // Search params: https://docs.curseforge.com/rest-api/?shell#search-mods
    pub fn search_mod<T>(&self, mut builder: T) -> SearchMods
    where T: FnMut(UrlBuilder) -> UrlBuilder
    {
        let url = UrlBuilder::new(format!("{}/search", self.base_url).as_str()).add_param("gameId", "432");
        let mut discover = Discover::new(builder(url).url.as_str());
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }

    // Get Mod Info
    pub fn get_mod(&self, mod_id: i32) -> GetMod {
        let url = format!("{}/mods/{}", self.base_url, mod_id);
        let mut discover = Discover::new(&*url);
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }

    // Get Mod Descriptions
    pub fn get_mod_desc(&self, mod_id: i32) -> Desc {
        let url = format!("{}/mods/{}/description", self.base_url, mod_id);
        let mut discover = Discover::new(&*url);
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }

    // Get Mod File
    pub fn get_mod_file(&self, mod_id: i32, file_id: i32) -> GetModFile {
        let url = format!("{}/mods/{}/files/{}", self.base_url, mod_id, file_id);
        let mut discover = Discover::new(&*url);
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }

    // Get Mod Files params: https://docs.curseforge.com/rest-api/?shell#get-mod-files
    pub fn get_mod_files<T>(&self, mod_id: i32, mut builder: T) -> GetModFiles
    where T: FnMut(UrlBuilder) -> UrlBuilder
    {
        let url = UrlBuilder::new(format!("{}/mods/{}/files", self.base_url, mod_id).as_str());
        let mut discover = Discover::new(builder(url).url.as_str());
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }

    // Get Mod File Changelog
    pub fn get_mod_file_changelog(&self, mod_id: i32, file_id: i32) -> Desc {
        let url = format!("{}/mods/{}/files/{}/changelog", self.base_url, mod_id, file_id);
        let mut discover = Discover::new(&*url);
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }

    // Get Mod File Link
    pub fn get_mod_file_link(&self, mod_id: i32, file_id: i32) -> Desc {
        let url = format!("{}/mods/{}/files/{}/download-url", self.base_url, mod_id, file_id);
        let mut discover = Discover::new(&*url);
        discover.set_curse_key(&self.api_key);
        discover.set_json_header();
        serde_json::from_str(&*discover.get()).unwrap()
    }
}

enum ModLoaderType {
    Any, Forge, LiteLoader, Fabric, Quilt, NeoForge
}

impl ModLoaderType {
    pub fn get(&self) -> &str {
        match self {
            ModLoaderType::Any => "0",
            ModLoaderType::Forge => "1",
            ModLoaderType::LiteLoader => "3",
            ModLoaderType::Fabric => "4",
            ModLoaderType::Quilt => "5",
            ModLoaderType::NeoForge => "6",
            _ => todo!(),
        }
    }
}