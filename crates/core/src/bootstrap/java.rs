use futures::future::join_all;
use std::env::consts::EXE_SUFFIX;
use std::env::{home_dir, var};
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

#[derive(Debug)]
pub struct JavaDistribution {
    pub install: JavaInstall,
    pub info: JavaInfo,
}

#[derive(Debug)]
pub struct JavaInfo {
    pub java_major_version: String, // java.specification.version e.g. 1.8
    pub jre_version: String,        // java.version e.g. 1.8.0_452
    pub implementor: String,        // java.vm.vendor e.g. Eclipse Adoptium
    pub java_runtime_version: String, // java.vm.version e.g. 24.0.1+9
    pub arch: String,               // os.arch e.g. x86
}

// The minium info to start the jvm
#[derive(Debug)]
pub struct JavaInstall {
    pub source: JavaSource, // For display usage
    pub path: String,       // the bin folder, does not contain the java executable
}

#[derive(Debug, Clone)]
pub enum JavaSource {
    JavaHome,
    PackageManager,
    Registry,
    User,
    Bundle,
}

const DEFAULT_INFO_STR: &str = "unknown";
const JAVA_EXECUTION_TIMEOUT: u64 = 5; // in seconds
fn get_java_executable_name() -> String {
    format!("java{EXE_SUFFIX}")
}

#[derive(Debug)]
pub enum JavaInfoError {
    Io(IoError),
    Parse(String),
    Timeout,
    Utf8Conversion(FromUtf8Error),
}

impl From<IoError> for JavaInfoError {
    fn from(err: IoError) -> Self {
        JavaInfoError::Io(err)
    }
}

impl From<FromUtf8Error> for JavaInfoError {
    fn from(err: FromUtf8Error) -> Self {
        JavaInfoError::Utf8Conversion(err)
    }
}

impl JavaDistribution {
    pub async fn get() -> Vec<JavaDistribution> {
        Self::from_installs(JavaInstall::get_all_java_installs()).await
    }

    async fn from_installs(installs: Vec<JavaInstall>) -> Vec<JavaDistribution> {
        let futures = installs.into_iter().map(|install| async {
            let executable = format!("{}java{}", install.path, EXE_SUFFIX);
            let info = match JavaInfo::parse_from_executable(&executable).await {
                Ok(info) => info,
                Err(e) => {
                    tracing::warn!("Failed to get Java info at {}: {:?}", install.path, e);
                    JavaInfo::default()
                }
            };

            JavaDistribution { install, info }
        });

        join_all(futures).await
    }
}

impl Default for JavaInfo {
    fn default() -> Self {
        Self {
            java_major_version: DEFAULT_INFO_STR.to_string(),
            jre_version: DEFAULT_INFO_STR.to_string(),
            implementor: DEFAULT_INFO_STR.to_string(),
            java_runtime_version: DEFAULT_INFO_STR.to_string(),
            arch: DEFAULT_INFO_STR.to_string(),
        }
    }
}

impl JavaInfo {
    async fn parse_from_executable(executable: &str) -> Result<Self, JavaInfoError> {
        let mut cmd = Command::new(executable);
        cmd.arg("-XshowSettings:properties");

        let output = timeout(Duration::from_secs(JAVA_EXECUTION_TIMEOUT), cmd.output())
            .await
            .map_err(|_| JavaInfoError::Timeout)??;

        let output_str = String::from_utf8(output.stderr)?;

        Self::parse_properties(&output_str)
    }

    fn parse_properties(output: &str) -> Result<Self, JavaInfoError> {
        let mut java_major_version = DEFAULT_INFO_STR.to_string();
        let mut jre_version = DEFAULT_INFO_STR.to_string();
        let mut implementor = DEFAULT_INFO_STR.to_string();
        let mut java_runtime_version = DEFAULT_INFO_STR.to_string();
        let mut arch = DEFAULT_INFO_STR.to_string();

        for line in output.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("java.specification.version") {
                java_major_version = parse_property_value(value);
            } else if let Some(value) = trimmed.strip_prefix("java.version") {
                jre_version = parse_property_value(value);
            } else if let Some(value) = trimmed.strip_prefix("java.vm.vendor") {
                implementor = parse_property_value(value);
            } else if let Some(value) = trimmed.strip_prefix("java.vm.version") {
                java_runtime_version = parse_property_value(value);
            } else if let Some(value) = trimmed.strip_prefix("os.arch") {
                arch = parse_property_value(value);
            }
        }

        Ok(Self {
            java_major_version,
            jre_version,
            implementor,
            java_runtime_version,
            arch,
        })
    }
}

fn parse_property_value(s: &str) -> String {
    s.split_once('=')
        .map(|(_, v)| v.trim())
        .unwrap_or("")
        .to_string()
}

impl JavaInstall {
    fn get_all_java_installs() -> Vec<Self> {
        let mut javas = Self::get_platform_java_distribution();
        if let Some(install) = Self::get_javahome_java_distribution() {
            javas.push(install);
        }
        javas
    }

    #[cfg(windows)]
    fn get_platform_java_distribution() -> Vec<Self> {
        const DISTRIBUTION_REGISRY_LOCATIONS: &[(&str, &str, &str)] = &[
            // Oracle JRE/JDK
            (
                "SOFTWARE\\JavaSoft\\Java Runtime Environment",
                "JavaHome",
                "",
            ),
            ("SOFTWARE\\JavaSoft\\Java Development Kit", "JavaHome", ""),
            // Oracle Java 9+
            ("SOFTWARE\\JavaSoft\\JRE", "JavaHome", ""),
            ("SOFTWARE\\JavaSoft\\JDK", "JavaHome", ""),
            // AdoptOpenJDK
            ("SOFTWARE\\AdoptOpenJDK\\JRE", "Path", "\\hotspot\\MSI"),
            ("SOFTWARE\\AdoptOpenJDK\\JDK", "Path", "\\hotspot\\MSI"),
            // Eclipse Foundation
            (
                "SOFTWARE\\Eclipse Foundation\\JDK",
                "Path",
                "\\hotspot\\MSI",
            ),
            // Eclipse Adoptium
            ("SOFTWARE\\Eclipse Adoptium\\JRE", "Path", "\\hotspot\\MSI"),
            ("SOFTWARE\\Eclipse Adoptium\\JDK", "Path", "\\hotspot\\MSI"),
            // IBM Semeru
            ("SOFTWARE\\Semeru\\JRE", "Path", "\\openj9\\MSI"),
            ("SOFTWARE\\Semeru\\JDK", "Path", "\\openj9\\MSI"),
            // Microsoft JDK
            ("SOFTWARE\\Microsoft\\JDK", "Path", "\\hotspot\\MSI"),
            // Azul Zulu
            ("SOFTWARE\\Azul Systems\\Zulu", "InstallationPath", ""),
            // BellSoft Liberica
            ("SOFTWARE\\BellSoft\\Liberica", "InstallationPath", ""),
        ];

        let mut javas = vec![];
        for &location in DISTRIBUTION_REGISRY_LOCATIONS {
            javas.extend(Self::get_java_distribution_from_registry(location));
        }

        javas.extend(Self::get_official_java_distribution_bundles(&[
            dirs_sys::known_folder_roaming_app_data()
                .unwrap()
                .join(".minecraft")
                .join("runtime"),
            dirs_sys::known_folder_local_app_data()
                .unwrap()
                .join("Packages")
                .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                .join("LocalCache")
                .join("Local")
                .join("runtime"),
        ]));
        javas
    }

    #[cfg(windows)]
    fn get_java_distribution_from_registry(location: (&str, &str, &str)) -> Vec<JavaInstall> {
        pub const ERROR_FILE_NOT_FOUND: u32 = 2u32;
        use winreg::RegKey;
        use winreg::enums::{
            HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ENUMERATE_SUB_KEYS, KEY_READ,
            KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        };
        let key_name = location.0;
        let value_name = location.1;
        let subkey_suffix = location.2;

        let mut javas = vec![];
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            for access in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
                let flags = KEY_READ | access | KEY_ENUMERATE_SUB_KEYS;
                let root = RegKey::predef(hive);

                let base_key = match root.open_subkey_with_flags(key_name, flags) {
                    Ok(k) => k,
                    Err(e) => {
                        if e.raw_os_error().unwrap_or(0) as u32 != ERROR_FILE_NOT_FOUND {
                            tracing::error!("Failed to open registry key {key_name} : {e}");
                        }
                        continue;
                    }
                };

                for subkey in base_key.enum_keys().filter_map(Result::ok) {
                    let key = match root.open_subkey_with_flags(
                        format!("{key_name}\\{subkey}{subkey_suffix}"),
                        flags,
                    ) {
                        Ok(k) => k,
                        Err(_) => continue,
                    };

                    if let Ok(dir) = key.get_value::<String, _>(value_name) {
                        javas.push(JavaInstall {
                            source: JavaSource::Registry,
                            path: Path::new(&dir).join("bin").to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
        javas
    }

    #[cfg(target_os = "linux")]
    fn get_platform_java_distribution() -> Vec<Self> {
        const AOSC_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib", "java-")]; // /usr/lib/java-<major>
        const DEBIAN_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib/jvm", "")]; // /usr/lib/jvm/java-<major>-openjdk-<arch>
        const FEDORA_JAVA_PATHS: [(&str, &str); 1] = [("/usr/lib/jvm", "")]; // /usr/lib/jvm/java-<major>-openjdk-<full_ver>.<fedora_major>.<arch>
        const GENTOO_JAVA_PATHS: [(&str, &str); 3] = [
            ("/usr/lib64", "openjdk-"), // /usr/lib64/openjdk-<major>
            ("/usr/lib", "openjdk-"),   // /usr/lib/openjdk-<major>
            ("/opt", "openjdk-bin-"),   // /opt/openjdk-bin-<ver>
        ];
        use crate::os::linux::get_os_release;
        let mut javas: Vec<JavaInstall> = vec![];
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
                        && bin_path.join(get_java_executable_name()).exists()
                    {
                        javas.push(JavaInstall {
                            source: JavaSource::PackageManager,
                            path: bin_path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
        javas.extend(Self::get_official_java_distribution_bundles(&[home_dir()
            .expect("Cannot find home dir")
            .join(".minecraft")
            .join("runtime")
            .join("java-runtime-delta")
            .join("linux")]));
        javas
    }

    #[cfg(target_os = "macos")]
    fn get_platform_java_distribution() -> Vec<Self> {
        todo!()
    }

    fn get_official_java_distribution_bundles(search_locations: &[PathBuf]) -> Vec<Self> {
        let mut javas = vec![];
        for location in search_locations {
            javas.extend(Self::find_java_in_dir(&location, JavaSource::Bundle))
        }
        javas
    }

    fn find_java_in_dir(path: &Path, java_source: JavaSource) -> Vec<Self> {
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
                    && bin_path.join(get_java_executable_name()).exists()
                {
                    javas.push(JavaInstall {
                        source: java_source.clone(),
                        path: bin_path.to_string_lossy().to_string(),
                    });
                }
            }
        }
        javas
    }

    fn get_javahome_java_distribution() -> Option<Self> {
        var("JAVA_HOME").ok().map(|path| Self {
            source: JavaSource::JavaHome,
            path: path + "/bin",
        })
    }

    fn get_executable_file_path(&self) -> Option<String> {
        Self::get_executable_file_path_from_path(&self.path)
    }

    fn get_executable_file_path_from_path<P: AsRef<Path>>(path: P) -> Option<String> {
        // todo We may prefer javaw?
        let executable = path.as_ref().join(get_java_executable_name());
        //TODO May be we should prove this file always exists.
        if executable.exists() {
            Some(executable.to_string_lossy().to_string())
        } else {
            None
        }
    }
}

#[tokio::test]
async fn test_java_detector() {
    println!("{:?}", JavaInstall::get_platform_java_distribution());
    if let Some(java_home) = var("JAVA_HOME").ok() {
        let path = format!("{}/bin/java", java_home);
        if let Ok(info) = JavaInfo::parse_from_executable(&path).await {
            assert!(!info.java_major_version.contains("unknown"));
        }
    }
}
