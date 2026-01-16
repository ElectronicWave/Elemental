use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct EnvPathProvider;

impl RuntimeProvider for EnvPathProvider {
    fn list(&self) -> Vec<PathBuf> {
        Self::get_official_java_distribution_bundles()
    }

    fn name(&self) -> &'static str {
        "Environment"
    }
}

impl EnvPathProvider {
    fn get_official_java_distribution_bundles() -> Vec<PathBuf> {
        let search_locations = Self::get_bundle_search_locations();
        let mut javas = vec![];
        for location in search_locations {
            javas.extend(Self::find_java_in_dir(&location));
        }
        javas
    }

    fn get_bundle_search_locations() -> Vec<PathBuf> {
        let mut locations = vec![];
        #[cfg(windows)]
        {
            use dirs_sys::known_folder_local_app_data;
            use dirs_sys::known_folder_roaming_app_data;
            locations.push(
                known_folder_roaming_app_data()
                    .unwrap()
                    .join(".minecraft")
                    .join("runtime"),
            );
            locations.push(
                known_folder_local_app_data()
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

    fn find_java_in_dir(path: &Path) -> Vec<PathBuf> {
        let mut javas = vec![];
        if !path.exists() || !path.is_dir() {
            return javas;
        }
        let Ok(read_dir) = path.read_dir() else {
            return javas;
        };
        for sub in read_dir {
            let Ok(sub_dir) = sub else {
                continue;
            };
            let sub_dir_path = sub_dir.path();
            if sub_dir_path.is_dir() {
                let bin_path = sub_dir_path.join("bin");
                if bin_path.exists()
                    && bin_path.is_dir()
                    && bin_path.join(format!("java{EXE_SUFFIX}")).exists()
                {
                    javas.push(sub_dir_path);
                }
            }
        }
        javas
    }
}
