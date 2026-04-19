use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaAssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    #[serde(rename = "totalSize")]
    pub total_size: usize,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaAssetIndexObjects {
    pub objects: HashMap<String, PistonMetaAssetIndexObject>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PistonMetaAssetIndexObject {
    pub hash: String,
    pub size: usize,
}
