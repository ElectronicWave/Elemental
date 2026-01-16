use std::path::PathBuf;

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct PackageManagerProvider;

impl RuntimeProvider for PackageManagerProvider {
    fn list(&self) -> Vec<PathBuf> {
        Self::get_platform_java_distribution()
    }

    fn name(&self) -> &'static str {
        "PackageManager"
    }
}

impl PackageManagerProvider {
    #[cfg(target_os = "linux")]
    fn get_platform_java_distribution() -> Vec<PathBuf> {
        const AOSC_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib", "java-")]; // /usr/lib/java-<major>
        const DEBIAN_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib/jvm", "")]; // /usr/lib/jvm/java-<major>-openjdk-<arch>
        const FEDORA_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib/jvm", "")]; // /usr/lib/jvm/java-<major>-openjdk-<full_ver>.<fedora_major>.<arch>
        const GENTOO_JAVA_PATHS: [(&str, &str); 3] = [
            ("/usr/lib64", "openjdk-"), // /usr/lib64/openjdk-<major>
            ("/usr/lib", "openjdk-"),   // /usr/lib/openjdk-<major>
            ("/opt", "openjdk-bin-"),   // /opt/openjdk-bin-<ver>
        ];
        use crate::os::linux::get_os_release;
        let mut javas: Vec<PathBuf> = vec![];
        let filters: &[(&str, &str)] = get_os_release()
            .and_then(|os_release| os_release.get("ID").map(|s| s.as_str()))
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
            if !path.exists() || !path.is_dir() {
                continue;
            }
            let Ok(read_dir) = path.read_dir() else {
                continue;
            };
            for sub in read_dir {
                let Ok(sub_dir) = sub else {
                    continue;
                };
                if !sub_dir.file_name().to_string_lossy().starts_with(filter.1) {
                    continue;
                }
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
        }
        javas
    }

    #[cfg(not(target_os = "linux"))]
    fn get_platform_java_distribution() -> Vec<PathBuf> {
        vec![]
    }
}
