use std::path::{Path, PathBuf};

use super::super::provider::RuntimeProvider;

#[derive(Default)]
pub struct RegistryProvider;

impl RuntimeProvider for RegistryProvider {
    fn list(&self) -> Vec<PathBuf> {
        Self::get_java_distribution_from_registry()
    }

    fn name(&self) -> &'static str {
        "Registry"
    }
}

impl RegistryProvider {
    #[cfg(windows)]
    fn get_java_distribution_from_registry() -> Vec<PathBuf> {
        const DISTRIBUTION_REGISTRY_LOCATIONS: &[(&str, &str, &str)] = &[
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
        for &location in DISTRIBUTION_REGISTRY_LOCATIONS {
            javas.extend(Self::get_java_from_location(location));
        }
        javas
    }

    #[cfg(windows)]
    fn get_java_from_location(location: (&str, &str, &str)) -> Vec<PathBuf> {
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
                        javas.push(Path::new(&dir).to_path_buf());
                    }
                }
            }
        }
        javas
    }

    #[cfg(not(windows))]
    fn get_java_distribution_from_registry() -> Vec<PathBuf> {
        vec![]
    }
}
