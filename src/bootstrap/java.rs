use crate::error::unification::UnifiedResult;
use log::{error, warn};
use std::{
    env::{consts::EXE_SUFFIX, var},
    io::Result,
    path::Path,
    process::Command,
};
use std::string::ToString;

#[derive(Debug)]
pub struct JavaDistribution {
    pub install: JavaInstall,
    pub info: JavaInfo,
}

#[derive(Debug)]
pub struct JavaInfo {
    pub java_major_version: String, // java.specification.version e.g. 1.8
    pub jre_version: String,        // java.version e.g. 1.8.0_452
    pub implememtor: String,        // java.vm.vendor e.g. Eclipse Adoptium
    pub java_runtime_version: String, // java.vm.version e.g. 24.0.1+9
    pub arch: String,               // os.arch e.g. x86
}

// The minium info to start the jvm
#[derive(Debug)]
pub struct JavaInstall {
    source: JavaSource, // For display usage
    path: String,        // the bin folder, does not contain the java executable
}

#[derive(Debug)]
pub enum JavaSource {
    JavaHome,
    PackageManager,
    Registry,
    User,
}

const DEFAULT_INFO_STRING: &str = "unknown";

impl JavaDistribution {
    pub fn from_installs(installs: Vec<JavaInstall>) -> Vec<JavaDistribution> {
        installs
            .into_iter()
            .filter_map(|install| {
                let executable = format!("{}java{}", install.path, EXE_SUFFIX);
                if let Err(e) = Command::new(&executable).arg("-version").output() {
                    warn!(
                        "Invalid java install located at {}! Error: {}",
                        install.path, e
                    );
                    return None;
                }
                let info = match JavaInfo::parse_from_executable_cmdl(&executable) {
                    Ok(info) => info,
                    Err(e) => {
                        warn!(
                            "Can not get the information of the java located at {}. Error: {}",
                            install.path, e
                        );
                        return Some(JavaDistribution {
                            install,
                            info: JavaInfo {
                                java_major_version: DEFAULT_INFO_STRING.to_string(),
                                jre_version: DEFAULT_INFO_STRING.to_string(),
                                implememtor: DEFAULT_INFO_STRING.to_string(),
                                java_runtime_version: DEFAULT_INFO_STRING.to_string(),
                                arch: DEFAULT_INFO_STRING.to_string(),
                            },
                        });
                    }
                };

                Some(JavaDistribution { install, info })
            })
            .collect()
    }
}

impl JavaInfo {
    fn parse_from_executable_cmdl(executable: &String) -> Result<Self> {
        let cmdl = Command::new(executable)
            .arg("-XshowSettings:properties")
            .arg("-version")
            .output()?;
        // FIXME!: -version return in stderr... and earlier java versions does not have --version option so we had to use the crappy -version
        let output = String::from_utf8(cmdl.stderr).to_stdio()?;

        let mut java_major_version = DEFAULT_INFO_STRING.to_string();
        let mut jre_version = DEFAULT_INFO_STRING.to_string();
        let mut implememtor = DEFAULT_INFO_STRING.to_string();
        let mut java_runtime_version = DEFAULT_INFO_STRING.to_string();
        let mut arch = DEFAULT_INFO_STRING.to_string();

        for line in output.lines() {
            let trimed = line.trim();
            if trimed.starts_with("java.specification.version = ") {
                java_major_version = trimed
                    .trim_start_matches("java.specification.version = ")
                    .to_string();
            } else if trimed.starts_with("java.version = ") {
                jre_version = trimed.trim_start_matches("java.version = ").to_string();
            } else if trimed.starts_with("java.vm.vendor = ") {
                implememtor = trimed.trim_start_matches("java.vm.vendor = ").to_string();
            } else if trimed.starts_with("java.vm.version = ") {
                java_runtime_version = trimed.trim_start_matches("java.vm.version = ").to_string();
            } else if trimed.starts_with("os.arch = ") {
                arch = trimed.trim_start_matches("os.arch = ").to_string();
            }
        }
        Ok(Self {
            java_major_version,
            jre_version,
            implememtor,
            java_runtime_version,
            arch,
        })
    }
}

impl JavaInstall {
    pub fn get_all_java_distribution() -> Vec<Self> {
        let mut javas = Self::get_platform_java_distribution();
        if let Some(install) = Self::get_javahome_java_distribution() {
            javas.push(install);
        }
        javas
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
                                source: JavaSource::Registry,
                                path: dir + "/bin",
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
            source: JavaSource::Registry,
            path: path + "/bin",
        })
    }

    pub fn get_executable_file_path(&self) -> Option<String> {
        Self::get_executable_file_path_from_path(&self.path)
    }

    pub(crate) fn get_executable_file_path_from_path<P: AsRef<Path>>(path: P) -> Option<String> {
        // todo We may prefer javaw?
        let filename = format!("java{}", EXE_SUFFIX);
        let executable = path.as_ref().join(filename);

        if executable.exists() {
            Some(executable.to_string_lossy().to_string())
        } else {
            None
        }
    }
}

#[test]
fn test_java_detector() {
    for distribution in JavaInstall::get_platform_java_distribution() {
        println!("{:?}", distribution)
    }
}
