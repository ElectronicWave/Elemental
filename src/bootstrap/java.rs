use log::error;
use std::{
    collections::HashMap,
    env::{consts::EXE_SUFFIX, var},
    fs::read_to_string,
    hash::RandomState,
    io::Result,
    path::Path,
    path::PathBuf,
    process::Command,
};

use crate::error::unification::UnifiedResult;

#[derive(Debug)]
pub struct JavaDistribution {
    pub install: JavaInstall,
    pub arch: u16,                             // EM_386, etc.
    pub release_info: Option<JavaReleaseInfo>, // It should exist
}

// The additional data
#[derive(Debug)]
pub struct JavaReleaseInfo {
    pub implememtor: Option<String>,
    pub implememtor_version: Option<String>,
    pub java_runtime_version: Option<String>,
}

// The minium info to start the jvm
#[derive(Debug)]
pub struct JavaInstall {
    id: String,
    path: PathBuf,
}

impl JavaReleaseInfo {
    pub fn parse_from_string(source: String) -> Self {
        let data: HashMap<&str, &str, RandomState> =
            HashMap::from_iter(source.lines().filter_map(|e| {
                if let Some((k, v)) = e.split_once("=") {
                    return Some((k, v.trim_start_matches('"').trim_end_matches('"')));
                }
                None
            }));

        Self {
            implememtor: data.get("IMPLEMENTOR").map(|s| s.to_string()),
            implememtor_version: data.get("IMPLEMENTOR_VERSION").map(|s| s.to_string()),
            java_runtime_version: data.get("JAVA_RUNTIME_VERSION").map(|s| s.to_string()),
        }
    }

    pub fn parse_from_release_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::parse_from_string(read_to_string(path)?))
    }

    pub fn parse_from_executable_cmdl(executable: String) -> Result<Self> {
        let cmdl = Command::new(executable)
            .arg("-XshowSettings:properties")
            .arg("-version")
            .output()?;
        let output = String::from_utf8(cmdl.stderr).to_stdio()?;
        //TODO Adapt More Java Ver Here

        // java.vm.version
        // java.vm.vendor
        // java.vendor.version
        let mut implememtor = String::new();
        let mut implememtor_version = String::new();
        let mut java_runtime_version = String::new();

        for line in output.lines() {
            let trimed = line.trim();
            if trimed.starts_with("java.vm.vendor = ") {
                implememtor = trimed.trim_start_matches("java.vm.vendor = ").to_string();
            } else if trimed.starts_with("java.vm.version = ") {
                java_runtime_version = trimed.trim_start_matches("java.vm.version = ").to_string();
            } else if trimed.starts_with("java.vendor.version") {
                implememtor_version = trimed
                    .trim_start_matches("java.vendor.version = ")
                    .to_string();
            }
        }
        Ok(Self {
            implememtor: Some(implememtor),
            implememtor_version: Some(implememtor_version),
            java_runtime_version: Some(java_runtime_version),
        })
    }

    pub fn try_parse<P: AsRef<Path>>(release: P, executable: String) -> Result<Self> {
        let result = Self::parse_from_release_file(release);
        if result.is_ok() {
            return result;
        }

        Self::parse_from_executable_cmdl(executable)
    }
}

impl JavaInstall {
    pub fn get_all_java_distribution() -> Vec<Self> {
        vec![]
    }

    #[cfg(windows)]
    pub fn get_platform_java_distribution() -> Vec<Self> {
        let mut javas = vec![];

        // Oracle
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\JavaSoft\\Java Runtime Environment",
            "JavaHome",
            "",
        ));
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\JavaSoft\\Java Development Kit",
            "JavaHome",
            "",
        ));
        // Oracle for Java 9 and newer
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\JavaSoft\\JRE",
            "JavaHome",
            "",
        ));
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\JavaSoft\\JDK",
            "JavaHome",
            "",
        ));
        // AdoptOpenJDK
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\AdoptOpenJDK\\JRE",
            "Path",
            "\\hotspot\\MSI",
        ));
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\AdoptOpenJDK\\JDK",
            "Path",
            "\\hotspot\\MSI",
        ));
        // Eclipse Foundation
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Eclipse Foundation\\JDK",
            "Path",
            "\\hotspot\\MSI",
        ));
        // Eclipse Adoptium
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Eclipse Adoptium\\JRE",
            "Path",
            "\\hotspot\\MSI",
        ));
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Eclipse Adoptium\\JDK",
            "Path",
            "\\hotspot\\MSI",
        ));
        // IBM Semeru
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Semeru\\JRE",
            "Path",
            "\\openj9\\MSI",
        ));
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Semeru\\JDK",
            "Path",
            "\\openj9\\MSI",
        ));
        // Microsoft JDK
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Microsoft\\JDK",
            "Path",
            "\\hotspot\\MSI",
        ));
        // Azul Zulu
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\Azul Systems\\Zulu",
            "InstallationPath",
            "",
        ));
        // BellSoft Liberica
        javas.extend(Self::get_java_distribution_from_registry(
            "SOFTWARE\\BellSoft\\Liberica",
            "InstallationPath",
            "",
        ));

        javas
    }

    #[cfg(windows)]
    fn get_java_distribution_from_registry(
        key_name: &str,
        key_java_dir: &str,
        subkey_suffix: &str,
    ) -> Vec<JavaInstall> {
        pub const ERROR_FILE_NOT_FOUND: u32 = 2u32;
        use winreg::RegKey;
        use winreg::enums::{
            HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ENUMERATE_SUB_KEYS, KEY_READ,
            KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        };

        let mut javas = vec![];
        for key_type in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            for root in [
                RegKey::predef(HKEY_CURRENT_USER),
                RegKey::predef(HKEY_LOCAL_MACHINE),
            ] {
                let flags = KEY_READ | key_type | KEY_ENUMERATE_SUB_KEYS;
                let base = match root.open_subkey_with_flags(key_name, flags) {
                    Ok(k) => k,
                    Err(e) => {
                        if e.raw_os_error().unwrap_or(0) as u32 != ERROR_FILE_NOT_FOUND {
                            error!("Failed to open registry key {} : {}", key_name, e);
                        }
                        continue;
                    }
                };

                for sub in base.enum_keys().filter_map(Result::ok) {
                    let full_path = format!("{}\\{}{}", key_name, sub, subkey_suffix);

                    let version = match root.open_subkey_with_flags(&full_path, KEY_READ | key_type)
                    {
                        Ok(k) => k,
                        Err(_) => continue,
                    };

                    match version.get_value::<String, _>(key_java_dir) {
                        Ok(dir) => {
                            javas.push(JavaInstall {
                                id: sub.clone(),
                                path: Path::new(&dir).join("bin").join("javaw.exe"),
                            });
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        javas
    }

    #[cfg(target_os = "linux")]
    pub fn get_platform_java_distribution() -> Vec<Self> {
        todo!()
    }

    #[cfg(target_os = "macos")]
    pub fn get_platform_java_distribution() -> Vec<Self> {
        todo!()
    }

    pub fn get_javahome_java_distribution() -> Option<Self> {
        var("JAVA_HOME").ok().map(|path| Self {
            id: "".to_string(),
            path: Path::new(&path).to_path_buf(),
        })
    }

    pub fn get_executable_file_path(&self) -> Option<String> {
        Self::get_executable_file_path_from_path(&self.path)
    }

    pub(crate) fn get_executable_file_path_from_path<P: AsRef<Path>>(path: P) -> Option<String> {
        let filename = format!("java{}", EXE_SUFFIX);
        let executable = path.as_ref().join("bin").join(filename);

        if executable.exists() {
            Some(executable.to_string_lossy().to_string())
        } else {
            None
        }
    }
}

#[test]
fn javahome() {
    println!(
        "{:?}",
        JavaInstall::get_javahome_java_distribution()
            .unwrap()
            .get_executable_file_path()
    );
}
