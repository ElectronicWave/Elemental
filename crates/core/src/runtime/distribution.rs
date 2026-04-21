use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

use futures::future::join_all;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::OnceCell;

use crate::runtime::provider::{RuntimeProvider, all_providers};

static DISTRIBUTION_CACHE: OnceCell<Vec<Distribution>> = OnceCell::const_new();

#[derive(Debug, Default, Clone)]
pub struct DistributionReleaseData {
    /// java.vm.vendor e.g. Eclipse Adoptium
    pub implementor: Option<String>,
    //? impl version may need ?
    /// os.arch e.g. x86_64
    pub architecture: Option<String>,
    /// java.specification.version e.g. 1.8
    pub major_version: Option<String>,
    /// java.version e.g. 1.8.0_452
    pub jre_version: Option<String>,
    /// java.vm.version e.g. 24.0.1+9
    pub jvm_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Distribution {
    // Release Data (when available)
    pub release: Option<DistributionReleaseData>,

    // Physical Data
    /// Path to the runtime installation
    pub path: PathBuf,

    // Discovery Data
    /// Elemental Provider used to discover this runtime
    pub provider: &'static str,
}

impl DistributionReleaseData {
    pub fn matches_java_major_version(&self, major_version: usize) -> bool {
        self.major_version
            .as_deref()
            .and_then(parse_java_major_version)
            .is_some_and(|current| current == major_version)
    }

    pub async fn parse_from_commandline(root: &Path) -> Self {
        let java_exe = root.join("bin").join("java");
        if let Ok(output) = Command::new(&java_exe)
            .arg("-XshowSettings:properties")
            .output()
            .await
        {
            if let Ok(stderr) = String::from_utf8(output.stderr) {
                Self::parse_properties(&stderr)
            } else {
                Self::default()
            }
        } else {
            Self::default()
        }
    }

    pub async fn parse_from_release(root: &Path) -> Self {
        let release_path = root.join("release");
        if let Ok(content) = fs::read_to_string(release_path).await {
            Self::parse_release_properties(&content)
        } else {
            Self::default()
        }
    }

    pub async fn parse(root: &Path) -> Self {
        if root.join("release").exists() {
            Self::parse_from_release(root).await
        } else {
            Self::parse_from_commandline(root).await
        }
    }

    pub fn is_lts(&self) -> bool {
        self.major_version
            .as_deref()
            .and_then(parse_java_major_version)
            .is_some_and(|version| matches!(version, 8 | 11 | 17 | 21))
    }

    fn parse_properties(output: &str) -> Self {
        let mut implementor = None;
        let mut architecture = None;
        let mut major_version = None;
        let mut jre_version = None;
        let mut jvm_version = None;

        for line in output.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("java.vm.vendor") {
                implementor = Some(parse_property_value(value));
            } else if let Some(value) = trimmed.strip_prefix("os.arch") {
                architecture = Some(parse_property_value(value));
            } else if let Some(value) = trimmed.strip_prefix("java.specification.version") {
                major_version = Some(parse_property_value(value));
            } else if let Some(value) = trimmed.strip_prefix("java.version") {
                jre_version = Some(parse_property_value(value));
            } else if let Some(value) = trimmed.strip_prefix("java.vm.version") {
                jvm_version = Some(parse_property_value(value));
            }
        }

        Self {
            implementor,
            architecture,
            major_version,
            jre_version,
            jvm_version,
        }
    }

    fn parse_release_properties(output: &str) -> Self {
        let mut implementor = None;
        let mut architecture = None;
        let mut major_version = None;
        let mut jre_version = None;
        let mut jvm_version = None;

        for line in output.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("IMPLEMENTOR=") {
                implementor = Some(parse_release_value(value));
            } else if let Some(value) = trimmed.strip_prefix("OS_ARCH=") {
                architecture = Some(parse_release_value(value));
            } else if let Some(value) = trimmed.strip_prefix("JAVA_VERSION=") {
                jre_version = Some(parse_release_value(value));
            } else if let Some(value) = trimmed.strip_prefix("JAVA_RUNTIME_VERSION=") {
                jvm_version = Some(parse_release_value(value));
            }
        }

        // Extract major version from jre_version
        if let Some(ref ver) = jre_version {
            major_version = parse_java_major_version(ver).map(|value| value.to_string());
        }

        Self {
            implementor,
            architecture,
            major_version,
            jre_version,
            jvm_version,
        }
    }
}

fn parse_property_value(s: &str) -> String {
    s.split_once('=')
        .map(|(_, v)| v.trim())
        .unwrap_or("")
        .to_string()
}

fn parse_release_value(s: &str) -> String {
    s.trim_matches('"').to_string()
}

impl Distribution {
    pub async fn build_from_root(root: PathBuf, provider: &'static str) -> Self {
        Self {
            release: Some(DistributionReleaseData::parse(&root).await),
            path: root,
            provider,
        }
    }

    pub async fn from_providers<O>(providers: Vec<Box<dyn RuntimeProvider>>) -> O
    where
        O: FromIterator<Self>,
    {
        let mut futures = Vec::new();
        for provider in providers {
            let name = provider.name();
            let paths = provider.list().await;
            for path in paths {
                futures.push(async move { Distribution::build_from_root(path, name).await });
            }
        }
        let distributions = join_all(futures).await;
        distributions.into_iter().collect()
    }

    pub async fn cached() -> Vec<Self> {
        DISTRIBUTION_CACHE
            .get_or_init(|| async { Distribution::from_providers::<Vec<_>>(all_providers()).await })
            .await
            .clone()
    }

    pub fn matches_java_major_version(&self, major_version: usize) -> bool {
        self.release
            .as_ref()
            .is_some_and(|release| release.matches_java_major_version(major_version))
    }

    pub async fn find_cached_by_java_major(major_version: usize) -> Option<Self> {
        Self::cached()
            .await
            .into_iter()
            .find(|distribution| distribution.matches_java_major_version(major_version))
    }

    pub fn executable(&self) -> PathBuf {
        self.path.join("bin").join(format!("java{}", EXE_SUFFIX))
    }
}

fn parse_java_major_version(version: &str) -> Option<usize> {
    let normalized = version.trim();
    if let Some(legacy) = normalized.strip_prefix("1.") {
        return legacy
            .split(['.', '_', '-'])
            .next()
            .and_then(|value| value.parse::<usize>().ok());
    }

    normalized
        .split(['.', '_', '-'])
        .next()
        .and_then(|value| value.parse::<usize>().ok())
}

#[tokio::test]
async fn test_distribution_build() {
    use super::provider::all_providers;
    let a: Vec<_> = Distribution::from_providers(all_providers()).await;
    println!("{:#?}", a);
}
