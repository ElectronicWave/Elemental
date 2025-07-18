use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMod {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMods {
    pub data: Vec<Data>,
    pub pagination: Pagination,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetModFile {
    pub data: File,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetModFiles {
    pub data: Vec<File>,
    pub pagination: Pagination,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Desc {
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub slug: String,
    pub links: Links,
    pub summary: String,
    pub status: i64,
    pub download_count: i64,
    pub is_featured: bool,
    pub primary_category_id: i64,
    pub categories: Vec<Category>,
    pub class_id: i64,
    pub authors: Vec<Author>,
    pub logo: Logo,
    pub screenshots: Vec<Screenshot>,
    pub main_file_id: i64,
    pub latest_files: Vec<LatestFile>,
    pub latest_files_indexes: Vec<LatestFilesIndex>,
    pub latest_early_access_files_indexes: Vec<LatestEarlyAccessFilesIndex>,
    pub date_created: String,
    pub date_modified: String,
    pub date_released: String,
    pub allow_mod_distribution: bool,
    pub game_popularity_rank: i64,
    pub is_available: bool,
    pub thumbs_up_count: i64,
    pub rating: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: i64,
    pub game_id: i64,
    pub mod_id: i64,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: i64,
    pub file_status: i64,
    pub hashes: Vec<Hash>,
    pub file_date: String,
    pub file_length: i64,
    pub download_count: i64,
    pub file_size_on_disk: i64,
    pub download_url: String,
    pub game_versions: Vec<String>,
    pub sortable_game_versions: Vec<SortableGameVersion>,
    pub dependencies: Vec<Dependency>,
    pub expose_as_alternative: bool,
    pub parent_project_file_id: i64,
    pub alternate_file_id: i64,
    pub is_server_pack: bool,
    pub server_pack_file_id: i64,
    pub is_early_access_content: bool,
    pub early_access_end_date: String,
    pub file_fingerprint: i64,
    pub modules: Vec<Module>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub website_url: String,
    pub wiki_url: String,
    pub issues_url: String,
    pub source_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub icon_url: String,
    pub date_modified: String,
    pub is_class: bool,
    pub class_id: i64,
    pub parent_category_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub id: i64,
    pub name: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Logo {
    pub id: i64,
    pub mod_id: i64,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub id: i64,
    pub mod_id: i64,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestFile {
    pub id: i64,
    pub game_id: i64,
    pub mod_id: i64,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: i64,
    pub file_status: i64,
    pub hashes: Vec<Hash>,
    pub file_date: String,
    pub file_length: i64,
    pub download_count: i64,
    pub file_size_on_disk: i64,
    pub download_url: String,
    pub game_versions: Vec<String>,
    pub sortable_game_versions: Vec<SortableGameVersion>,
    pub dependencies: Vec<Dependency>,
    pub expose_as_alternative: bool,
    pub parent_project_file_id: i64,
    pub alternate_file_id: i64,
    pub is_server_pack: bool,
    pub server_pack_file_id: i64,
    pub is_early_access_content: bool,
    pub early_access_end_date: String,
    pub file_fingerprint: i64,
    pub modules: Vec<Module>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub value: String,
    pub algo: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: String,
    pub game_version: String,
    pub game_version_release_date: String,
    pub game_version_type_id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub mod_id: i64,
    pub relation_type: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub name: String,
    pub fingerprint: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestFilesIndex {
    pub game_version: String,
    pub file_id: i64,
    pub filename: String,
    pub release_type: i64,
    pub game_version_type_id: i64,
    pub mod_loader: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestEarlyAccessFilesIndex {
    pub game_version: String,
    pub file_id: i64,
    pub filename: String,
    pub release_type: i64,
    pub game_version_type_id: i64,
    pub mod_loader: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: i64,
    pub page_size: i64,
    pub result_count: i64,
    pub total_count: i64,
}