use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::super::provider::RuntimeProvider;
use crate::runtime::discovery::{find_runtime_homes, is_runtime_home, path_exists};

const MINECRAFT_RUNTIME_SCAN_DEPTH: usize = 4;
const USER_TOOLCHAIN_SCAN_DEPTH: usize = 3;

#[derive(Default)]
pub struct EnvPathProvider;

#[async_trait]
impl RuntimeProvider for EnvPathProvider {
    async fn list(&self) -> Vec<PathBuf> {
        let mut javas = Self::find_java_in_path().await;
        javas.extend(Self::get_user_toolchain_java_distribution().await);
        javas.extend(Self::get_official_java_distribution_bundles().await);
        javas
    }

    fn name(&self) -> &'static str {
        "Environment"
    }
}

impl EnvPathProvider {
    async fn find_java_in_path() -> Vec<PathBuf> {
        let Some(path) = std::env::var_os("PATH") else {
            return Vec::new();
        };

        let mut javas = Vec::new();
        for directory in std::env::split_paths(&path) {
            if directory.as_os_str().is_empty() {
                continue;
            }
            #[cfg(windows)]
            if Self::is_oracle_javapath_shim(&directory) {
                continue;
            }
            if let Some(runtime_home) = Self::runtime_home_from_path_entry(&directory).await {
                javas.push(runtime_home);
            }
        }
        javas
    }

    async fn runtime_home_from_path_entry(directory: &Path) -> Option<PathBuf> {
        if is_runtime_home(directory).await {
            return Some(directory.to_path_buf());
        }

        let executable = directory.join(format!("java{EXE_SUFFIX}"));
        if !path_exists(&executable).await {
            return None;
        }

        let is_bin_directory = directory
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("bin"));
        if !is_bin_directory {
            return None;
        }

        directory.parent().map(Path::to_path_buf)
    }

    async fn get_user_toolchain_java_distribution() -> Vec<PathBuf> {
        let locations = Self::get_user_toolchain_search_locations();
        let mut javas = Vec::new();
        for location in locations {
            javas.extend(find_runtime_homes(&location, USER_TOOLCHAIN_SCAN_DEPTH).await);
        }
        javas
    }

    fn get_user_toolchain_search_locations() -> Vec<PathBuf> {
        let Some(home) = dirs::home_dir() else {
            tracing::warn!("can't determine home directory for Java toolchain search");
            return Vec::new();
        };

        let sdkman_root = std::env::var_os("SDKMAN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".sdkman"));
        let asdf_root = std::env::var_os("ASDF_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".asdf"));
        let gradle_root = std::env::var_os("GRADLE_USER_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".gradle"));

        let mut locations = Vec::new();
        locations.extend([
            home.join(".jdks"),
            sdkman_root.join("candidates").join("java"),
            asdf_root.join("installs").join("java"),
            gradle_root.join("jdks"),
        ]);

        #[cfg(target_os = "macos")]
        {
            locations.push(
                home.join("Library")
                    .join("Java")
                    .join("JavaVirtualMachines"),
            );
        }

        locations
    }

    async fn get_official_java_distribution_bundles() -> Vec<PathBuf> {
        let search_locations = Self::get_bundle_search_locations();
        let mut javas = Vec::new();
        for location in search_locations {
            javas.extend(find_runtime_homes(&location, MINECRAFT_RUNTIME_SCAN_DEPTH).await);
        }
        javas
    }

    fn get_bundle_search_locations() -> Vec<PathBuf> {
        let mut locations = Vec::new();

        #[cfg(windows)]
        {
            if let Some(data_dir) = dirs::data_dir() {
                locations.push(data_dir.join(".minecraft").join("runtime"));
            }
            if let Some(local_data_dir) = dirs::data_local_dir() {
                locations.push(local_data_dir.join(".minecraft").join("runtime"));
                locations.push(
                    local_data_dir
                        .join("Packages")
                        .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                        .join("LocalCache")
                        .join("Local")
                        .join("runtime"),
                );
            }
            if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                locations.push(
                    PathBuf::from(program_files_x86)
                        .join("Minecraft Launcher")
                        .join("runtime"),
                );
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                locations.push(home.join(".minecraft").join("runtime"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                locations.push(
                    home.join("Library")
                        .join("Application Support")
                        .join("minecraft")
                        .join("runtime"),
                );
            }
        }

        locations
    }

    #[cfg(windows)]
    fn is_oracle_javapath_shim(path: &Path) -> bool {
        let normalized = path
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        normalized.contains("\\common files\\oracle\\java\\")
    }
}
