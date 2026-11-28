mod apply;
mod storage;

use elemental_schema::mojang::piston::{
    PistonMetaData, PistonMetaGenericArgument, PistonMetaLibraries, PistonMetaLogging,
};
use serde::{Deserialize, Serialize};

pub use apply::{apply_metadata_patches, apply_metadata_patches_traced};
pub(crate) use storage::baseline_path;
pub use storage::{apply_patches_with_mode, apply_scoped_patches, read_base_metadata};

/// How metadata patches are applied to a version JSON instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchMode {
    /// Virtual patch (`vpatch`): overlays patches in memory without touching `<instance>.json`.
    #[default]
    Virtual,
    /// Physical patch (`patch`): backs up the clean baseline and writes merged metadata to `<instance>.json`.
    Physical,
}

/// Declarative transformation applied on top of a base [`PistonMetaData`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataPatch {
    #[serde(default)]
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_library_prefixes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_libraries: Vec<PistonMetaLibraries>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub game_arguments: Vec<PistonMetaGenericArgument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jvm_arguments: Vec<PistonMetaGenericArgument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<PistonMetaLogging>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_major_version: Option<usize>,
}

/// A [`MetadataPatch`] identified by its owning addon or layer ID and effective mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedMetadataPatch {
    pub id: String,
    #[serde(default)]
    pub effective_mode: PatchMode,
    pub patch: MetadataPatch,
}

/// Effective diff produced by applying a single [`NamedMetadataPatch`] step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataPatchStepDiff {
    pub id: String,
    pub priority: i32,
    pub effective_mode: PatchMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_class: Option<(String, String)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaced_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_game_arguments: Vec<PistonMetaGenericArgument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_jvm_arguments: Vec<PistonMetaGenericArgument>,
    pub logging_changed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_major_version: Option<(usize, usize)>,
}

/// Merged [`PistonMetaData`] paired with per-step patch diffs in execution order.
#[derive(Debug, Clone)]
pub struct PatchTrace {
    pub metadata: PistonMetaData,
    pub steps: Vec<MetadataPatchStepDiff>,
}
