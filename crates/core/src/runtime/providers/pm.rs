use std::path::PathBuf;

use async_trait::async_trait;

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct PackageManagerProvider;
#[async_trait]
impl RuntimeProvider for PackageManagerProvider {
    async fn list(&self) -> Vec<PathBuf> {
        Self::get_platform_java_distribution().await
    }

    fn name(&self) -> &'static str {
        "PackageManager"
    }
}

impl PackageManagerProvider {
    #[cfg(target_os = "linux")]
    async fn get_platform_java_distribution() -> Vec<PathBuf> {
        const AOSC_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib", "java-")]; // /usr/lib/java-<major>
        const DEBIAN_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib/jvm", "")]; // /usr/lib/jvm/java-<major>-openjdk-<arch>
        const FEDORA_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib/jvm", "")]; // /usr/lib/jvm/java-<major>-openjdk-<full_ver>.<fedora_major>.<arch>
        const GENTOO_JAVA_PATHS: [(&str, &str); 3] = [
            ("/usr/lib64", "openjdk-"), // /usr/lib64/openjdk-<major>
            ("/usr/lib", "openjdk-"),   // /usr/lib/openjdk-<major>
            ("/opt", "openjdk-bin-"),   // /opt/openjdk-bin-<ver>
        ];
        let mut javas: Vec<PathBuf> = vec![];
        let os_release = rs_release::get_os_release().unwrap_or_default();

        let filters: &[(&str, &str)] =
            os_release
                .get("ID")
                .map(|s| s.as_str())
                .map_or(&[], |os_id| match os_id {
                    "aosc" => &AOSC_JAVA_PATHS,
                    "debian" | "ubuntu" => &DEBIAN_JAVA_PATHS,
                    "fedora" => &FEDORA_JAVA_PATHS,
                    "gentoo" => &GENTOO_JAVA_PATHS as &[(&str, &str)],
                    "Deepin" | "deepin" => todo!("deepin implementation"),
                    _ => &[],
                });
        for filter in filters {
            let path = Path::new(filter.0);
            if !tokio::fs::try_exists(path).await.unwrap_or(false) {
                continue;
            }
            let Ok(mut read_dir) = tokio::fs::read_dir(path).await else {
                continue;
            };
            while let Ok(Some(sub)) = read_dir.next_entry().await {
                if !sub.file_name().to_string_lossy().starts_with(filter.1) {
                    continue;
                }
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
        }
        javas
    }

    #[cfg(not(target_os = "linux"))]
    async fn get_platform_java_distribution() -> Vec<PathBuf> {
        vec![]
    }
}
