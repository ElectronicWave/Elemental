use std::{
    collections::HashMap,
    env::consts::{ARCH, OS},
};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct MojangBaseUrl {
    // It seems still being used by the official launcher, but all known endpoints can be replaced by piston-meta
    // launchermeta may be just exist for compatibility reason
    // Maybe we should remove it?
    // FIXME: More research required
    #[deprecated]
    pub launchermeta: String,
    // For `meta` stuff, e.g. Client.json, version manifest, assets index, etc.
    pub meta: String,
    // For `data` stuff, e.g. client.jar
    pub data: String,
    // For assets. e.g. lang files, icons
    pub resources: String,
    // Mojang maven, for libraries
    pub libraries: String,
}

impl Default for MojangBaseUrl {
    fn default() -> Self {
        Self {
            launchermeta: "launchermeta.mojang.com".to_owned(),
            meta: "piston-meta.mojang.com".to_owned(),
            resources: "resources.download.minecraft.net".to_owned(),
            libraries: "libraries.minecraft.net".to_owned(),
            data: "piston-data.mojang.com".to_owned(),
        }
    }
}

impl MojangBaseUrl {
    pub fn get_object_url(&self, hash: String) -> String {
        format!(
            "https://{}/{}/{hash}",
            self.resources,
            hash.get(0..2).unwrap()
        )
    }
}

/// https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
// Note this mojang does not provide every version
// For instance, experimental snapshots, 2.0 April fool versions...
// We may need to add extra source or hardcode
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionManifestData {
    pub latest: LatestVersionData,
    pub versions: Vec<VersionMetaData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LatestVersionData {
    pub release: String,
    pub snapshot: String,
}

// This provides where to download the client.json and its version id
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionMetaData {
    pub id: String,
    #[serde(rename = "type")]
    pub release_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: usize,
}

/// https://piston-meta.mojang.com/v1/packages/<sha1>/<id>.json
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionData {
    pub arguments: MinecraftArguments, // FIXME: Only exist on >1.12.2, so maybe should be set to Option?
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>, // <=1.12.2
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub assets: String, // It seems same as assetIndex.id
    #[serde(rename = "complianceLevel")]
    pub compliance_level: usize,// FIXME: may not exist
    pub downloads: PistonMetaDownloads,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: PistonMetaJavaVersion,
    pub libraries: Vec<PistonMetaLibraries>,
    pub logging: PistonMetaLogging,// FIXME: may not exist
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: usize,
    #[serde(rename = "type")]
    pub release_type: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MinecraftArguments {
    pub game: Vec<PistonMetaGenericArgument>,
    pub jvm: Vec<PistonMetaGenericArgument>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PistonMetaGenericArgument {
    Plain(String),
    Rule(PistonMetaRuleArgument),
}

impl MinecraftArguments {
    pub fn get_all_arguments(&self) -> Vec<String> {
        let mut arguments = vec![];
        arguments.extend(Self::concat_generic_arguments(&self.jvm));
        arguments.extend(Self::concat_generic_arguments(&self.game));
        arguments
    }
    pub fn get_jvm_arguments(&self) -> Vec<String> {
        Self::concat_generic_arguments(&self.jvm)
    }

    pub fn get_game_arguments(&self) -> Vec<String> {
        Self::concat_generic_arguments(&self.game)
    }
    pub fn concat_generic_arguments(arguments: &Vec<PistonMetaGenericArgument>) -> Vec<String> {
        let mut result = vec![];

        for arg in arguments {
            match arg {
                PistonMetaGenericArgument::Plain(s) => {
                    result.push(s.clone());
                }
                PistonMetaGenericArgument::Rule(piston_meta_rule_argument) => {
                    if piston_meta_rule_argument
                        .rules
                        .iter()
                        .all(|rule| rule.is_allow())
                    {
                        if let Some(val) = &piston_meta_rule_argument.value {
                            match val {
                                ContinuousArgument::Single(value) => {
                                    result.push(value.clone());
                                }
                                ContinuousArgument::Multi(items) => {
                                    result.extend(items.clone());
                                }
                            };
                        }
                    }
                }
            };
        }
        result
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaRuleArgument {
    pub rules: Vec<PistonMetaRuleArgumentRules>,// FIXME: May not exist
    pub value: Option<ContinuousArgument>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaRuleArgumentRules {
    pub action: String,
    // is_demo_user, has_custom_resolution, has_quick_plays_support, is_quick_play_singleplayer, is_quick_play_multiplayer, is_quick_play_realms
    // Note: not Optional when for game args
    // Note: only exist on game arg rules
    pub features: Option<HashMap<String, bool>>, //TODO impl features
    // Note: not Optional when for jvm args
    // Note: only exist on jvm rules & libraries download rules
    pub os: Option<OperatingSystem>,
}

impl PistonMetaRuleArgumentRules {
    pub fn is_allow(&self) -> bool {
        let mut action = self.action == "allow";

        //TODO features here...
        if let Some(_) = self.features {
            return false;
        }

        if let Some(os) = &self.os {
            if !os.is_fit() {
                action = !action;
            }
        }

        action
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct OperatingSystem {
    pub arch: Option<String>,// Note: not exist for libraries download rules
    pub name: Option<String>,// Note: must exist for libraries download rules
    pub version: Option<String>,
}

impl OperatingSystem {
    pub fn is_fit(&self) -> bool {
        if let Some(name) = &self.name {
            if (name == "osx" || name == "macosx") && OS != "macos" {
                return false;
            }

            if name != OS {
                return false;
            }
        }

        if let Some(arch) = &self.arch {
            if arch != ARCH {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContinuousArgument {
    Single(String),
    Multi(Vec<String>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    // size may be too small
    pub size: usize,
    #[serde(rename = "totalSize")]
    pub total_size: usize,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaDownloads {
    pub client: PistonMetaDownload,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaDownload {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaJavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLibraries {
    pub downloads: PistonMetaLibrariesDownloads,
    pub name: String,
    // linux, osx, windows (All optional)
    pub natives: Option<HashMap<String, String>>,
    pub rules: Option<Vec<PistonMetaRuleArgumentRules>>,
    pub extract: Option<PistonMetaLibrariesExtract>,
}

impl PistonMetaLibraries {
    // classifiers
    pub fn try_get_classifiers_native_artifact(
        &self,
    ) -> Option<&PistonMetaLibrariesDownloadsArtifact> {
        if let Some(classifiers) = &self.downloads.classifiers {
            if let Some(keys) = &self.natives {
                if let Some(key) = keys.get(OS) {
                    return classifiers.get(key);
                } else if OS == "macos" {
                    if let Some(key) = keys.get("osx") {
                        return classifiers.get(key);
                    }
                }
            }
        }

        None
    }

    // Latest Version
    pub fn try_get_native_artifact(&self) -> Option<&PistonMetaLibrariesDownloadsArtifact> {
        let artifact = &self.downloads.artifact;
        if artifact.path.ends_with(&format!("-natives-{}.jar", OS))
            || OS == "macos" && artifact.path.ends_with("-natives-osx.jar")
        {
            return Some(artifact);
        }
        None
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLibrariesExtract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLibrariesDownloads {
    pub artifact: PistonMetaLibrariesDownloadsArtifact,//FIXME: may not exist
    // linux-x86_64, natives-linux, natives-macos, natives-windows, natives-osx, natives-windows-32, natives-windows-64
    pub classifiers: Option<HashMap<String, PistonMetaLibrariesDownloadsArtifact>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLibrariesDownloadsArtifact {
    pub sha1: String,
    pub size: usize,
    pub url: String,
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLogging {
    pub client: PistonMetaLoggingSide,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLoggingSide {
    pub argument: String,
    pub file: PistonMetaLoggingSideFile,
    #[serde(rename = "type")]
    pub logging_type: String,// Only `log4j2-xml`
}
#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLoggingSideFile {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaAssetIndexObjects {
    pub objects: HashMap<String, PistonMetaAssetIndexObject>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaAssetIndexObject {
    pub hash: String,
    pub size: usize,
}
