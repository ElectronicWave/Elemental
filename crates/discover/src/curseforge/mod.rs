use std::io::stdout;
use crate::curseforge::serialize::{Desc, GetMod, GetModFile, GetModFiles, SearchMods};
use crate::{Discover, UrlBuilder, CF_API, CF_API_SPECIAL};

pub mod serialize;

#[derive(Debug)]
pub struct Curse {
    base_url: String,
    api_key: String,
    special_mode: bool
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
            base_url: if special_mode {
                CF_API_SPECIAL.to_string()
            } else {
                CF_API.to_string()
            },
            api_key: "$2a$10$m.XJaH.ysAKZ3VzWIfnlP.RXkN54zQSYIUxBM8/H5riHQ2cUx3koy".to_string(), // Default API key
            special_mode
        }
    }

    pub fn new(api_key: String, special_mode: bool) -> Self {
        Self {
            base_url: if special_mode {
                CF_API_SPECIAL.to_string()
            } else {
                CF_API.to_string()
            },
            api_key,
            special_mode
        }
    }

    // Search params: https://docs.curseforge.com/rest-api/?shell#search-mods
    pub fn search_mod<T>(&self, mut builder: T) -> SearchMods
    where T: FnMut(UrlBuilder) -> UrlBuilder
    {
        let url = UrlBuilder::new(format!("{}/search", self.base_url).as_str()).add_param("gameId", "432");
        let mut discover = self.get_discover(builder(url).url.as_str());
        serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    // Get Mod Info
    pub fn get_mod(&self, mod_id: i32) -> GetMod {
        let url = format!("{}/mods/{}", self.base_url, mod_id);
        let mut discover = self.get_discover(&*url);
        println!("{:?}", str::from_utf8(&*discover.get()));
        GetMod::default()
        //serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    // Get Mod Descriptions
    pub fn get_mod_desc(&self, mod_id: i32) -> Desc {
        let url = format!("{}/mods/{}/description", self.base_url, mod_id);
        let mut discover = self.get_discover(&*url);
        serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    // Get Mod File
    pub fn get_mod_file(&self, mod_id: i32, file_id: i32) -> GetModFile {
        let url = format!("{}/mods/{}/files/{}", self.base_url, mod_id, file_id);
        let mut discover = self.get_discover(&*url);
        serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    // Get Mod Files params: https://docs.curseforge.com/rest-api/?shell#get-mod-files
    pub fn get_mod_files<T>(&self, mod_id: i32, mut builder: T) -> GetModFiles
    where T: FnMut(UrlBuilder) -> UrlBuilder
    {
        let url = UrlBuilder::new(format!("{}/mods/{}/files", self.base_url, mod_id).as_str());
        let mut discover = self.get_discover(builder(url).url.as_str());
        serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    // Get Mod File Changelog
    pub fn get_mod_file_changelog(&self, mod_id: i32, file_id: i32) -> Desc {
        let url = format!("{}/mods/{}/files/{}/changelog", self.base_url, mod_id, file_id);
        let mut discover = self.get_discover(&*url);
        serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    // Get Mod File Link
    pub fn get_mod_file_link(&self, mod_id: i32, file_id: i32) -> Desc {
        let url = format!("{}/mods/{}/files/{}/download-url", self.base_url, mod_id, file_id);
        let mut discover = self.get_discover(&*url);
        serde_json::from_str(str::from_utf8(&*discover.get()).unwrap()).unwrap()
    }

    fn get_discover(&self, url: &str) -> Discover{
        let mut discover = Discover::new(url);
        discover.set_curse_key(&self.api_key);
        if self.special_mode {
            discover.easy_client.useragent("Packust/1.0.0").unwrap();
        }
        discover
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