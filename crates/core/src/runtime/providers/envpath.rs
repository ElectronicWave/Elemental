use async_trait::async_trait;
use dirs::data_local_dir;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct EnvPathProvider;

#[async_trait]
impl RuntimeProvider for EnvPathProvider {
    async fn list(&self) -> Vec<PathBuf> {
        Self::get_official_java_distribution_bundles().await
    }

    fn name(&self) -> &'static str {
        "Environment"
    }
}

impl EnvPathProvider {
    async fn get_official_java_distribution_bundles() -> Vec<PathBuf> {
        let search_locations = Self::get_bundle_search_locations();
        let mut javas = vec![];
        for location in search_locations {
            javas.extend(Self::find_java_in_dir(&location).await);
        }
        javas
    }

    fn get_bundle_search_locations() -> Vec<PathBuf> {
        let mut locations = vec![];
        #[cfg(windows)]
        {
            locations.push(data_local_dir().unwrap().join(".minecraft").join("runtime"));
            locations.push(
                data_local_dir()
                    .unwrap()
                    .join("Packages")
                    .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                    .join("LocalCache")
                    .join("Local")
                    .join("runtime"),
            );
        }
        #[cfg(target_os = "linux")]
        {
            locations.push(
                home_dir()
                    .expect("Cannot find home dir")
                    .join(".minecraft")
                    .join("runtime")
                    .join("java-runtime-delta")
                    .join("linux"),
            );
        }
        locations
    }

    async fn find_java_in_dir(path: &Path) -> Vec<PathBuf> {
        let mut javas = vec![];
        if !tokio::fs::try_exists(path).await.unwrap_or(false) {
            return javas;
        }
        let Ok(mut read_dir) = tokio::fs::read_dir(path).await else {
            return javas;
        };
        while let Ok(Some(sub)) = read_dir.next_entry().await {
            let sub_dir_path = sub.path();
            if sub_dir_path.is_dir() {
                let bin_path = sub_dir_path.join("bin");
                let java_exe = bin_path.join(format!("java{EXE_SUFFIX}"));
                if tokio::fs::try_exists(&bin_path).await.unwrap_or(false)
                    && tokio::fs::try_exists(&java_exe).await.unwrap_or(false)
                {
                    javas.push(sub_dir_path);
                }
            }
        }
        javas
    }
}
