use serde::{Deserialize, Serialize};
use crate::meta::loader::ModLoader;

#[derive(Deserialize, Serialize)]
pub struct Metadata {
    pub filename: String,
    pub name: String,
    pub side: String,
    pub loader: Option<ModLoader>,
    pub links: Option<Vec<MetaLink>>,
}

#[derive(Deserialize, Serialize)]
pub struct MetaLink {
    pub platform: String,
    pub url: String,
    pub version: String
}

impl Metadata {
    pub fn new(filename: String, name: String, side: String, loader: Option<ModLoader>, links: Option<Vec<MetaLink>>) -> Self {
        Metadata {
            filename,
            name,
            side,
            loader,
            links
        }
    }

    pub fn serialize(&self) -> String {
        toml::to_string(self).unwrap()
    }
}