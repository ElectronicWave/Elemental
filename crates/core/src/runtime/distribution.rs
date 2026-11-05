use std::collections::HashSet;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::future::join_all;
use futures::stream::{self, StreamExt};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::OnceCell;

use crate::runtime::{
    error::{Error, Result},
    provider::{RuntimeProvider, runtime_providers},
};

static DISTRIBUTION_CACHE: OnceCell<Vec<Distribution>> = OnceCell::const_new();
const JAVA_INFO_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const JAVA_INFO_PROBE_CONCURRENCY: usize = 8;
const JAVA_PROBE_ENV_VARIABLES: &[&str] = &[
    "JAVA_HOME",
    "JRE_HOME",
    "JAVA_ARGS",
    "CLASSPATH",
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
];

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

    /// Explicit executable path when launch should use a specific java/javaw binary
    pub executable_override: Option<PathBuf>,

    // Discovery Data
    /// Elemental Provider used to discover this runtime
    pub provider: &'static str,
}

impl DistributionReleaseData {
    pub fn java_major_version(&self) -> Option<usize> {
        self.major_version
            .as_deref()
            .and_then(Self::parse_java_major_version)
    }

    pub fn matches_java_major_version(&self, major_version: usize) -> bool {
        self.java_major_version()
            .is_some_and(|current| current == major_version)
    }

    pub async fn parse_from_commandline(root: &Path) -> Result<Self> {
        let java_exe = root.join("bin").join(format!("java{EXE_SUFFIX}"));
        let mut command = Command::new(&java_exe);
        command.arg("-XshowSettings:properties").kill_on_drop(true);
        for variable_name in JAVA_PROBE_ENV_VARIABLES {
            command.env_remove(variable_name);
        }

        let output = tokio::time::timeout(JAVA_INFO_PROBE_TIMEOUT, command.output())
            .await
            .map_err(|_| Error::ProbeTimeout {
                executable: java_exe.clone(),
                timeout_secs: JAVA_INFO_PROBE_TIMEOUT.as_secs(),
            })?
            .map_err(|source| Error::ProbeSpawn {
                executable: java_exe.clone(),
                source,
            })?;

        if !output.status.success() {
            return Err(Error::ProbeFailed {
                executable: java_exe,
                status: output.status,
            });
        }

        let release = Self::parse_properties(&String::from_utf8_lossy(&output.stderr));
        if release.java_major_version().is_none() {
            return Err(Error::ProbeMissingVersion {
                executable: java_exe,
            });
        }

        Ok(release)
    }

    pub async fn parse_from_release(root: &Path) -> Result<Self> {
        let release_path = root.join("release");
        let content =
            fs::read_to_string(&release_path)
                .await
                .map_err(|source| Error::ReadRelease {
                    path: release_path.clone(),
                    source,
                })?;
        let release = Self::parse_release_properties(&content);
        if release.java_major_version().is_none() {
            return Err(Error::ReleaseMissingVersion { path: release_path });
        }

        Ok(release)
    }

    pub async fn parse(root: &Path) -> Result<Self> {
        let release_path = root.join("release");
        let release_exists =
            fs::try_exists(&release_path)
                .await
                .map_err(|source| Error::CheckRelease {
                    path: release_path.clone(),
                    source,
                })?;

        if release_exists {
            Self::parse_from_release(root).await
        } else {
            Self::parse_from_commandline(root).await
        }
    }

    pub fn is_lts(&self) -> bool {
        self.major_version
            .as_deref()
            .and_then(Self::parse_java_major_version)
            .is_some_and(|version| matches!(version, 8 | 11 | 17 | 21))
    }

    fn parse_properties(output: &str) -> Self {
        let mut implementor = None;
        let mut architecture = None;
        let mut major_version = None;
        let mut jre_version = None;
        let mut jvm_version = None;

        for (key, value) in Self::property_pairs(output) {
            match key {
                "java.vm.vendor" => implementor = Some(value.to_string()),
                "os.arch" => architecture = Some(value.to_string()),
                "java.specification.version" => major_version = Some(value.to_string()),
                "java.version" => jre_version = Some(value.to_string()),
                "java.vm.version" => jvm_version = Some(value.to_string()),
                _ => {}
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

        for (key, value) in Self::property_pairs(output) {
            let value = value.trim_matches('"');
            match key {
                "IMPLEMENTOR" => implementor = Some(value.to_string()),
                "OS_ARCH" => architecture = Some(value.to_string()),
                "JAVA_VERSION" => jre_version = Some(value.to_string()),
                "JAVA_RUNTIME_VERSION" => jvm_version = Some(value.to_string()),
                _ => {}
            }
        }

        // Extract major version from jre_version
        if let Some(ref ver) = jre_version {
            major_version = Self::parse_java_major_version(ver).map(|value| value.to_string());
        }

        Self {
            implementor,
            architecture,
            major_version,
            jre_version,
            jvm_version,
        }
    }

    fn property_pairs(output: &str) -> impl Iterator<Item = (&str, &str)> {
        output.lines().filter_map(|line| {
            let (key, value) = line.trim().split_once('=')?;
            Some((key.trim(), value.trim()))
        })
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
}

impl Distribution {
    pub async fn build_from_root(root: PathBuf, provider: &'static str) -> Result<Self> {
        let requested_root = root.clone();
        let root =
            fs::canonicalize(&root)
                .await
                .map_err(|source| Error::CanonicalizeRuntimeRoot {
                    path: root.clone(),
                    source,
                })?;
        let release = DistributionReleaseData::parse(&root)
            .await
            .map_err(|source| Error::DetermineRuntimeProperties {
                path: requested_root,
                source: Box::new(source),
            })?;

        Ok(Self {
            release: Some(release),
            path: root,
            executable_override: None,
            provider,
        })
    }

    pub async fn build_from_executable(
        executable: PathBuf,
        provider: &'static str,
    ) -> Result<Self> {
        let normalized_executable = Self::normalize_executable_path(executable)?;
        let normalized_executable =
            fs::canonicalize(&normalized_executable)
                .await
                .map_err(|source| Error::CanonicalizeExecutable {
                    path: normalized_executable.clone(),
                    source,
                })?;
        let runtime_root = Self::infer_runtime_root(&normalized_executable)?;

        let release = DistributionReleaseData::parse(&runtime_root)
            .await
            .map_err(|source| Error::DetermineRuntimeProperties {
                path: runtime_root.clone(),
                source: Box::new(source),
            })?;

        Ok(Self {
            release: Some(release),
            path: runtime_root,
            executable_override: Some(normalized_executable),
            provider,
        })
    }

    pub async fn from_providers<O>(providers: Vec<std::sync::Arc<dyn RuntimeProvider>>) -> O
    where
        O: FromIterator<Self>,
    {
        let provider_lists = join_all(
            providers
                .into_iter()
                .map(|provider| async move { (provider.name(), provider.list().await) }),
        )
        .await;

        let mut probes = Vec::new();
        for (name, paths) in provider_lists {
            for path in paths {
                let candidate_path = path.clone();
                probes.push(async move {
                    Distribution::build_from_root(path, name)
                        .await
                        .map_err(|source| Error::BuildDistribution {
                            provider: name,
                            path: candidate_path,
                            source: Box::new(source),
                        })
                });
            }
        }

        let distributions: Vec<Self> = stream::iter(probes)
            .buffer_unordered(JAVA_INFO_PROBE_CONCURRENCY)
            .filter_map(|result| async move {
                match result {
                    Ok(distribution) => Some(distribution),
                    Err(error) => {
                        tracing::warn!(%error, "skip invalid Java runtime candidate");
                        None
                    }
                }
            })
            .collect()
            .await;

        let mut seen_paths = HashSet::with_capacity(distributions.len());
        let mut unique_distributions: Vec<Self> = distributions
            .into_iter()
            .filter(|distribution| seen_paths.insert(distribution.path.clone()))
            .collect();
        unique_distributions.sort_by(ordering::compare_distributions);
        unique_distributions.into_iter().collect()
    }

    pub async fn cached() -> Vec<Self> {
        DISTRIBUTION_CACHE
            .get_or_init(|| async {
                Distribution::from_providers::<Vec<_>>(runtime_providers()).await
            })
            .await
            .clone()
    }

    pub fn matches_java_major_version(&self, major_version: usize) -> bool {
        self.release
            .as_ref()
            .is_some_and(|release| release.matches_java_major_version(major_version))
    }

    pub fn java_major_version(&self) -> Option<usize> {
        self.release
            .as_ref()
            .and_then(DistributionReleaseData::java_major_version)
    }

    pub async fn find_cached_by_java_major(major_version: usize) -> Option<Self> {
        Self::cached()
            .await
            .into_iter()
            .find(|distribution| distribution.matches_java_major_version(major_version))
    }

    pub fn executable(&self) -> PathBuf {
        self.executable_override
            .clone()
            .unwrap_or_else(|| self.path.join("bin").join(format!("java{}", EXE_SUFFIX)))
    }

    fn normalize_executable_path(executable: PathBuf) -> Result<PathBuf> {
        let normalized = if executable.is_absolute() {
            executable
        } else {
            std::env::current_dir()
                .map_err(|source| Error::CurrentDir { source })?
                .join(executable)
        };

        if !normalized.is_file() {
            return Err(Error::ExecutableNotFile { path: normalized });
        }

        let executable_name = normalized
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| Error::ExecutableNoStem {
                path: normalized.clone(),
            })?;
        if executable_name != "java" && executable_name != "javaw" {
            return Err(Error::ExecutableNotJava { path: normalized });
        }

        Ok(normalized)
    }

    fn infer_runtime_root(executable: &Path) -> Result<PathBuf> {
        let bin_directory = executable
            .parent()
            .ok_or_else(|| Error::ExecutableNoParent {
                path: executable.to_path_buf(),
            })?;
        let runtime_root = bin_directory
            .parent()
            .ok_or_else(|| Error::ExecutableNotInBin {
                path: executable.to_path_buf(),
            })?;

        Ok(runtime_root.to_path_buf())
    }
}

mod ordering {
    use std::cmp::Ordering;

    use super::Distribution;

    pub(super) fn compare_distributions(left: &Distribution, right: &Distribution) -> Ordering {
        let left_version = left
            .release
            .as_ref()
            .and_then(|release| release.jre_version.as_deref());
        let right_version = right
            .release
            .as_ref()
            .and_then(|release| release.jre_version.as_deref());

        right
            .java_major_version()
            .cmp(&left.java_major_version())
            .then_with(|| compare_release_versions(right_version, left_version))
            .then_with(|| left.path.cmp(&right.path))
    }

    fn compare_release_versions(left: Option<&str>, right: Option<&str>) -> Ordering {
        match (left, right) {
            (Some(left), Some(right)) => {
                let mut left_components = version_components(left).peekable();
                let mut right_components = version_components(right).peekable();
                if left_components.peek().is_none() || right_components.peek().is_none() {
                    left.cmp(right)
                } else {
                    left_components.cmp(right_components)
                }
            }
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
    }

    fn version_components(version: &str) -> impl Iterator<Item = u64> + '_ {
        version
            .split(|character: char| !character.is_ascii_digit())
            .filter(|component| !component.is_empty())
            .filter_map(|component| component.parse::<u64>().ok())
    }
}

#[tokio::test]
async fn default_providers_return_deduplicated_distributions_sorted_by_major_version() {
    use super::provider::default_providers;

    let distributions: Vec<_> = Distribution::from_providers(default_providers()).await;
    let mut paths: Vec<_> = distributions
        .iter()
        .map(|distribution| distribution.path.clone())
        .collect();
    let path_count = paths.len();
    paths.sort();
    paths.dedup();

    assert_eq!(paths.len(), path_count);
    assert!(
        distributions
            .iter()
            .all(|distribution| distribution.java_major_version().is_some())
    );
    assert!(
        distributions
            .windows(2)
            .all(|pair| { pair[0].java_major_version() >= pair[1].java_major_version() })
    );
    println!("{:#?}", distributions);
}

#[test]
fn parse_properties_uses_exact_keys_and_unquotes_release_values() {
    let diagnostic = r#"
        java.vm.vendor = Eclipse Adoptium
        os.arch = amd64
        java.specification.version = 21
        java.version = 21.0.5
        java.version.date = 2024-10-15
        java.vm.version = 21.0.5+11-LTS
    "#;
    let release = DistributionReleaseData::parse_properties(diagnostic);
    assert_eq!(release.jre_version.as_deref(), Some("21.0.5"));
    assert_eq!(release.jvm_version.as_deref(), Some("21.0.5+11-LTS"));
    assert_eq!(release.java_major_version(), Some(21));

    let release_file = r#"
        IMPLEMENTOR="Eclipse Adoptium"
        OS_ARCH="x86_64"
        JAVA_VERSION="1.8.0_452"
        JAVA_VERSION_DATE="2025-01-21"
        JAVA_RUNTIME_VERSION="1.8.0_452-b09"
    "#;
    let release = DistributionReleaseData::parse_release_properties(release_file);
    assert_eq!(release.implementor.as_deref(), Some("Eclipse Adoptium"));
    assert_eq!(release.jre_version.as_deref(), Some("1.8.0_452"));
    assert_eq!(release.java_major_version(), Some(8));
}
