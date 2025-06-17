use std::{
    collections::HashMap,
    env::consts::{ARCH, OS},
};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct MojangBaseUrl {
    pub launchermeta: String,
    pub launchermeta_https: bool,
    pub pistonmeta: String,
    pub pistondata: String,
    pub resources: String,
    pub libraries: String,
}

impl Default for MojangBaseUrl {
    fn default() -> Self {
        Self {
            launchermeta: "launchermeta.mojang.com".to_owned(),
            launchermeta_https: false,
            pistonmeta: "piston-meta.mojang.com".to_owned(),
            resources: "resources.download.minecraft.net".to_owned(),
            libraries: "libraries.minecraft.net".to_owned(),
            pistondata: "piston-data.mojang.com".to_owned(),
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

/// http://launchermeta.mojang.com/mc/game/version_manifest.json
#[derive(Debug, Deserialize, Serialize)]
pub struct LaunchMetaData {
    pub latest: LaunchMetaLatestData,
    pub versions: Vec<LaunchMetaVersionData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LaunchMetaLatestData {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LaunchMetaVersionData {
    pub id: String,
    #[serde(rename = "type")]
    pub typo: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

/// https://piston-meta.mojang.com/v1/packages/<->/<->.json
#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaData {
    pub arguments: PistonMetaArguments,
    #[serde(rename = "assetIndex")]
    pub asset_index: PistonMetaAssetIndex,
    pub assets: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: usize,
    pub downloads: PistonMetaDownloads,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: PistonMetaJavaVersion,
    pub libraries: Vec<PistonMetaLibraries>,
    pub logging: PistonMetaLogging,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: usize,
    #[serde(rename = "type")]
    pub typo: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaArguments {
    pub game: Vec<PistonMetaGenericArgument>,
    pub jvm: Vec<PistonMetaGenericArgument>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PistonMetaGenericArgument {
    Plain(String),
    Rule(PistonMetaRuleArgument),
}

impl PistonMetaArguments {
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
    pub rules: Vec<PistonMetaRuleArgumentRules>,
    pub value: Option<ContinuousArgument>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaRuleArgumentRules {
    pub action: String,
    pub features: Option<HashMap<String, bool>>, //TODO impl features
    pub os: Option<OperatingSystem>,
}

impl PistonMetaRuleArgumentRules {
    pub fn is_allow(&self) -> bool {
        let mut action = self.action == "allow";
        if let Some(os) = &self.os {
            if !os.is_fit() {
                action = !action;
            }
        }

        //TODO features here...

        action
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct OperatingSystem {
    pub arch: Option<String>,
    pub name: Option<String>,
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
pub struct PistonMetaAssetIndex {
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
    pub server: PistonMetaDownload,
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
    pub artifact: PistonMetaLibrariesDownloadsArtifact,
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
    // TODO? server: ...
}
#[derive(Debug, Deserialize, Serialize)]
pub struct PistonMetaLoggingSide {
    pub argument: String,
    pub file: PistonMetaLoggingSideFile,
    #[serde(rename = "type")]
    pub typo: String,
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
