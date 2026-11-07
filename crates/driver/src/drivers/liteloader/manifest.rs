use std::collections::HashMap;

use serde::Deserialize;

mod release;

pub(super) use release::{collect_releases, select_build};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteLoaderRelease {
    pub game_version: String,
    pub loader_version: String,
    pub stream: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum LiteLoaderLibraryResolution {
    #[default]
    Release,
    Snapshot,
}

#[derive(Debug, Clone)]
pub(super) struct LiteLoaderSelectedBuild {
    pub(super) game_version: String,
    pub(super) repository: LiteLoaderManifestRepository,
    pub(super) common_libraries: Vec<LiteLoaderManifestLibrary>,
    pub(super) build: LiteLoaderManifestBuild,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifest {
    pub(super) versions: HashMap<String, LiteLoaderManifestGameVersion>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestGameVersion {
    pub(super) repo: LiteLoaderManifestRepository,
    #[serde(default)]
    pub(super) artefacts: HashMap<String, HashMap<String, LiteLoaderManifestBuild>>,
    pub(super) snapshots: Option<LiteLoaderManifestSnapshots>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestRepository {
    pub(super) stream: LiteLoaderRepositoryStream,
    #[serde(rename = "type")]
    _repository_type: String,
    pub(super) url: String,
    #[serde(rename = "classifier")]
    _classifier: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(super) enum LiteLoaderRepositoryStream {
    #[serde(rename = "RELEASE")]
    Release,
    #[serde(rename = "SNAPSHOT")]
    Snapshot,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestSnapshots {
    #[serde(default)]
    pub(super) libraries: Vec<LiteLoaderManifestLibrary>,
    #[serde(flatten)]
    pub(super) packages: HashMap<String, HashMap<String, LiteLoaderManifestBuild>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestBuild {
    #[serde(rename = "tweakClass")]
    pub(super) tweak_class: String,
    #[serde(default)]
    pub(super) libraries: Vec<LiteLoaderManifestLibrary>,
    pub(super) stream: String,
    pub(super) file: String,
    pub(super) version: String,
    #[serde(rename = "build")]
    _build: Option<String>,
    #[serde(rename = "md5")]
    _md5: Option<String>,
    pub(super) timestamp: Option<String>,
    #[serde(rename = "lastSuccessfulBuild")]
    _last_successful_build: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiteLoaderManifestLibrary {
    pub(super) name: String,
    #[serde(default)]
    pub(super) url: String,
    #[serde(skip)]
    pub(super) resolution: LiteLoaderLibraryResolution,
}

impl LiteLoaderManifestLibrary {
    fn with_resolution(mut self, resolution: LiteLoaderLibraryResolution) -> Self {
        self.resolution = resolution;
        self
    }
}

impl LiteLoaderManifestBuild {
    fn with_library_resolution(mut self, resolution: LiteLoaderLibraryResolution) -> Self {
        for library in &mut self.libraries {
            library.resolution = resolution;
        }
        self
    }
}
