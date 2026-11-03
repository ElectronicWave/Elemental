#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::Path;
use std::path::PathBuf;

use async_trait::async_trait;

use super::super::provider::RuntimeProvider;
use crate::runtime::discovery::find_runtime_homes;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::runtime::discovery::find_runtime_homes_with_prefixes;

const PLATFORM_SCAN_DEPTH: usize = 1;

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
    #[cfg(windows)]
    async fn get_platform_java_distribution() -> Vec<PathBuf> {
        const VENDOR_DIRECTORIES: &[&str] = &[
            "Java",
            "BellSoft",
            "AdoptOpenJDK",
            "Zulu",
            "Microsoft",
            "Eclipse Foundation",
            "Eclipse Adoptium",
            "Semeru",
            "Amazon Corretto",
            "SapMachine",
        ];

        let mut javas = Vec::new();
        for variable_name in ["ProgramFiles", "ProgramFiles(x86)"] {
            let Some(base_path) = std::env::var_os(variable_name).map(PathBuf::from) else {
                continue;
            };
            for vendor_directory in VENDOR_DIRECTORIES {
                javas.extend(
                    find_runtime_homes(&base_path.join(vendor_directory), PLATFORM_SCAN_DEPTH)
                        .await,
                );
            }
        }
        javas
    }

    #[cfg(target_os = "linux")]
    async fn get_platform_java_distribution() -> Vec<PathBuf> {
        let mut javas = Vec::new();

        for directory in [
            "/usr/java",
            "/usr/lib/jvm",
            "/usr/lib32/jvm",
            "/usr/lib64/jvm",
            "/opt/jdk",
            "/opt/jdks",
            "/opt/ibm",
            "/app/jdk",
        ] {
            javas.extend(find_runtime_homes(Path::new(directory), PLATFORM_SCAN_DEPTH).await);
        }

        javas.extend(
            find_runtime_homes_with_prefixes(
                Path::new("/usr/lib64"),
                PLATFORM_SCAN_DEPTH,
                &["openjdk-", "openj9-"],
            )
            .await,
        );
        javas.extend(
            find_runtime_homes_with_prefixes(
                Path::new("/usr/lib"),
                PLATFORM_SCAN_DEPTH,
                &["java", "openjdk-", "openj9-"],
            )
            .await,
        );
        javas.extend(
            find_runtime_homes_with_prefixes(
                Path::new("/opt"),
                PLATFORM_SCAN_DEPTH,
                &["openjdk-", "openj9-"],
            )
            .await,
        );

        if let Some(snap_root) = std::env::var_os("SNAP").map(PathBuf::from) {
            for directory in ["usr/lib/jvm", "app/jdk"] {
                javas.extend(
                    find_runtime_homes(&snap_root.join(directory), PLATFORM_SCAN_DEPTH).await,
                );
            }
        }

        javas
    }

    #[cfg(target_os = "macos")]
    async fn get_platform_java_distribution() -> Vec<PathBuf> {
        let mut javas = Vec::new();

        for directory in [
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ] {
            javas.extend(find_runtime_homes(Path::new(directory), 3).await);
        }
        for directory in ["/opt/homebrew/opt", "/usr/local/opt"] {
            javas.extend(
                find_runtime_homes_with_prefixes(
                    Path::new(directory),
                    PLATFORM_SCAN_DEPTH,
                    &["openjdk", "java"],
                )
                .await,
            );
        }

        for directory in [
            "/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home",
            "/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java",
        ] {
            javas.extend(find_runtime_homes(Path::new(directory), 0).await);
        }

        javas
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    async fn get_platform_java_distribution() -> Vec<PathBuf> {
        Vec::new()
    }
}
