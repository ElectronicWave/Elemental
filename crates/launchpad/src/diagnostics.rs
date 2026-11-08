use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use anyhow::Result;
use elemental::{
    core::launcher::command::LaunchCommand,
    launcher::{Launcher, PreparedInstanceInfo},
};

pub struct VersionDiagnostics {
    pub version_root: PathBuf,
    pub metadata_path: PathBuf,
    pub version_jar_path: PathBuf,
    pub natives_root: PathBuf,
    pub natives_root_binaries: Vec<PathBuf>,
    pub natives_nested_binaries: Vec<PathBuf>,
    pub metadata_id: String,
    pub inherited_game_version: Option<String>,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
}

pub struct LaunchDiagnostics<'a> {
    pub driver_name: &'a str,
    pub loader_version: Option<&'a str>,
    pub instance_name: &'a str,
    pub game_version: &'a str,
    pub prepared_ms: u128,
    pub install_status: &'a dyn Debug,
    pub runtime_executable: &'a Path,
    pub diagnostics: &'a VersionDiagnostics,
    pub command: &'a LaunchCommand,
}

pub struct LaunchSummary<'a> {
    pub driver_name: &'a str,
    pub game_version: &'a str,
    pub loader_version: Option<&'a str>,
    pub runtime_executable: &'a Path,
    pub version_root: &'a Path,
    pub install_status: &'a dyn Debug,
    pub prepared_ms: u128,
    pub exit_status: ExitStatus,
}

pub fn collect_version_diagnostics(info: &PreparedInstanceInfo) -> VersionDiagnostics {
    let natives_root_binaries = info
        .natives_files
        .iter()
        .filter(|path| path.parent() == Some(info.natives_directory.as_path()))
        .cloned()
        .collect();
    let natives_nested_binaries = info
        .natives_files
        .iter()
        .filter(|path| is_native_binary(path))
        .cloned()
        .collect();

    VersionDiagnostics {
        version_root: info.version_root.clone(),
        metadata_path: info.metadata_path.clone(),
        version_jar_path: info.version_jar.clone(),
        natives_root: info.natives_directory.clone(),
        natives_root_binaries,
        natives_nested_binaries,
        metadata_id: info.metadata.id.clone(),
        inherited_game_version: info.metadata.inherits_from.clone(),
        main_class: info.metadata.main_class.clone(),
        classpath: info.classpath.clone(),
    }
}

pub async fn run_process(launcher: &Launcher, command: LaunchCommand) -> Result<ExitStatus> {
    let mut child = launcher.launch(command)?.child;
    Ok(child.wait().await?)
}

pub fn print_launch_diagnostics(diagnostics_input: &LaunchDiagnostics<'_>) {
    let command_arguments = diagnostics_input.command.arguments();

    println!("instance: {}", diagnostics_input.instance_name);
    println!("driver: {}", diagnostics_input.driver_name);
    println!("game version: {}", diagnostics_input.game_version);
    if let Some(loader_version) = diagnostics_input.loader_version {
        println!("loader version: {}", loader_version);
    }
    println!("metadata id: {}", diagnostics_input.diagnostics.metadata_id);
    println!(
        "metadata inherits_from: {}",
        diagnostics_input
            .diagnostics
            .inherited_game_version
            .as_deref()
            .unwrap_or("<none>")
    );
    println!(
        "metadata main class: {}",
        diagnostics_input.diagnostics.main_class
    );
    println!(
        "Using java executable: {}",
        diagnostics_input.runtime_executable.display()
    );
    println!(
        "command executable: {}",
        diagnostics_input.command.program().display()
    );
    println!(
        "command cwd: {}",
        diagnostics_input
            .command
            .cwd
            .as_ref()
            .map_or_else(|| "<none>".to_owned(), |cwd| cwd.display().to_string())
    );
    println!("command args count: {}", command_arguments.len());
    println!(
        "version root: {}",
        diagnostics_input.diagnostics.version_root.display()
    );
    println!(
        "metadata path: {}",
        diagnostics_input.diagnostics.metadata_path.display()
    );
    println!(
        "version jar: {}",
        diagnostics_input.diagnostics.version_jar_path.display()
    );
    println!(
        "natives root: {}",
        diagnostics_input.diagnostics.natives_root.display()
    );
    println!(
        "natives root exists: {}",
        diagnostics_input.diagnostics.natives_root.exists()
    );
    println!(
        "natives root file count: {}",
        diagnostics_input.diagnostics.natives_root_binaries.len()
    );
    println!(
        "natives recursive binary count: {}",
        diagnostics_input.diagnostics.natives_nested_binaries.len()
    );
    println!(
        "natives root files: {}",
        format_path_list(
            &diagnostics_input.diagnostics.natives_root_binaries,
            &diagnostics_input.diagnostics.natives_root
        )
    );
    println!(
        "natives recursive binaries: {}",
        format_path_list(
            &diagnostics_input.diagnostics.natives_nested_binaries,
            &diagnostics_input.diagnostics.natives_root
        )
    );
    println!(
        "natives probe exists (lwjgl): {}",
        diagnostics_input
            .diagnostics
            .natives_root
            .join(expected_lwjgl_binary_name())
            .exists()
    );
    println!("install status: {:?}", diagnostics_input.install_status);
    println!("prepared in {}ms", diagnostics_input.prepared_ms);

    for prefix in [
        "-Djava.library.path=",
        "-Dorg.lwjgl.librarypath=",
        "-Dorg.lwjgl.system.SharedLibraryExtractPath=",
        "-Djna.tmpdir=",
        "-Dio.netty.native.workdir=",
    ] {
        if let Some(argument) = find_argument_with_prefix(&command_arguments, prefix) {
            println!("launch arg {}{}", prefix, argument);
        }
    }

    if !diagnostics_input.diagnostics.classpath.is_empty() {
        println!(
            "classpath entries: {}",
            diagnostics_input.diagnostics.classpath.len()
        );
        println!(
            "classpath preview: {}",
            diagnostics_input
                .diagnostics
                .classpath
                .iter()
                .take(8)
                .map(|entry| entry.display().to_string())
                .collect::<Vec<String>>()
                .join(" | ")
        );
    }
}

pub fn print_summary(summary: &LaunchSummary<'_>) {
    println!(
        "Using java executable: {}",
        summary.runtime_executable.display()
    );
    println!("version root: {}", summary.version_root.display());
    println!("driver: {}", summary.driver_name);
    println!("game version: {}", summary.game_version);
    if let Some(loader_version) = summary.loader_version {
        println!("loader version: {}", loader_version);
    }
    println!("install status: {:?}", summary.install_status);
    println!("prepared in {}ms", summary.prepared_ms);
    println!("process exited with: {}", summary.exit_status);
}

fn is_native_binary(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "dll" | "so" | "dylib" | "jnilib"
            )
        })
}

fn format_path_list(paths: &[PathBuf], root: &Path) -> String {
    if paths.is_empty() {
        return "<none>".to_owned();
    }

    paths
        .iter()
        .map(|path| {
            path.strip_prefix(root).map_or_else(
                |_| path.display().to_string(),
                |relative| relative.display().to_string(),
            )
        })
        .collect::<Vec<String>>()
        .join(", ")
}

fn expected_lwjgl_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "lwjgl.dll"
    } else if cfg!(target_os = "macos") {
        "liblwjgl.dylib"
    } else {
        "liblwjgl.so"
    }
}

fn find_argument_with_prefix<'a>(arguments: &'a [String], prefix: &str) -> Option<&'a str> {
    arguments
        .iter()
        .find_map(|argument| argument.strip_prefix(prefix))
}
