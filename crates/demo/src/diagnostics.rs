use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use anyhow::{Context, Result};
use elemental::{
    core::{
        launcher::{command::LaunchCommand, process},
        storage::Storage,
    },
    driver::families::version_json::{
        VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonVersionStorageExt,
    },
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
}

pub fn collect_version_diagnostics<L, VL>(
    version: &Storage<VL, Storage<L>>,
) -> Result<VersionDiagnostics>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let metadata = version.metadata()?;
    let version_root = version.path.clone();
    let metadata_path = version.metadata_path()?;
    let version_jar_path = version.jar_path()?;
    let natives_root = version.platform_natives_path();
    let natives_root_binaries = collect_root_files(&natives_root)?;
    let natives_nested_binaries = collect_recursive_native_files(&natives_root)?;

    Ok(VersionDiagnostics {
        version_root,
        metadata_path,
        version_jar_path,
        natives_root,
        natives_root_binaries,
        natives_nested_binaries,
        metadata_id: metadata.id,
        inherited_game_version: metadata.inherits_from,
        main_class: metadata.main_class,
    })
}

pub async fn run_logged_process(command: LaunchCommand) -> Result<ExitStatus> {
    let mut launched = process::spawn_command_logged(command)?;
    let mut lines = launched.lines;
    let log_task = tokio::spawn(async move {
        while let Some(line) = lines.recv().await {
            println!("[{:?}] {}", line.source, line.line);
        }
    });
    let exit_status = launched.child.wait().await?;
    log_task.await.context("join process log task failed")?;
    Ok(exit_status)
}

pub fn print_launch_diagnostics(
    driver_name: &str,
    loader_version: Option<&str>,
    instance_name: &str,
    game_version: &str,
    prepared_ms: u128,
    install_status: &impl std::fmt::Debug,
    runtime_executable: &Path,
    diagnostics: &VersionDiagnostics,
    command: &LaunchCommand,
) {
    println!("instance: {}", instance_name);
    println!("driver: {}", driver_name);
    println!("game version: {}", game_version);
    if let Some(loader_version) = loader_version {
        println!("loader version: {}", loader_version);
    }
    println!("metadata id: {}", diagnostics.metadata_id);
    println!(
        "metadata inherits_from: {}",
        diagnostics
            .inherited_game_version
            .as_deref()
            .unwrap_or("<none>")
    );
    println!("metadata main class: {}", diagnostics.main_class);
    println!("Using java executable: {}", runtime_executable.display());
    println!("command executable: {}", command.program.display());
    println!(
        "command cwd: {}",
        command
            .cwd
            .as_ref()
            .map_or_else(|| "<none>".to_owned(), |cwd| cwd.display().to_string())
    );
    println!("command args count: {}", command.args.len());
    println!("version root: {}", diagnostics.version_root.display());
    println!("metadata path: {}", diagnostics.metadata_path.display());
    println!("version jar: {}", diagnostics.version_jar_path.display());
    println!("natives root: {}", diagnostics.natives_root.display());
    println!("natives root exists: {}", diagnostics.natives_root.exists());
    println!(
        "natives root file count: {}",
        diagnostics.natives_root_binaries.len()
    );
    println!(
        "natives recursive binary count: {}",
        diagnostics.natives_nested_binaries.len()
    );
    println!(
        "natives root files: {}",
        format_path_list(
            &diagnostics.natives_root_binaries,
            &diagnostics.natives_root
        )
    );
    println!(
        "natives recursive binaries: {}",
        format_path_list(
            &diagnostics.natives_nested_binaries,
            &diagnostics.natives_root
        )
    );
    println!(
        "natives probe exists (lwjgl): {}",
        diagnostics
            .natives_root
            .join(expected_lwjgl_binary_name())
            .exists()
    );
    println!("install status: {:?}", install_status);
    println!("prepared in {}ms", prepared_ms);

    for prefix in [
        "-Djava.library.path=",
        "-Dorg.lwjgl.librarypath=",
        "-Dorg.lwjgl.system.SharedLibraryExtractPath=",
        "-Djna.tmpdir=",
        "-Dio.netty.native.workdir=",
    ] {
        if let Some(argument) = find_argument_with_prefix(&command.args, prefix) {
            println!("launch arg {}{}", prefix, argument);
        }
    }

    if let Some(classpath_entries) = classpath_entries(&command.args) {
        println!("classpath entries: {}", classpath_entries.len());
        println!(
            "classpath preview: {}",
            classpath_entries
                .iter()
                .take(8)
                .map(|entry| entry.as_str())
                .collect::<Vec<&str>>()
                .join(" | ")
        );
    }
}

pub fn print_summary(
    driver_name: &str,
    game_version: &str,
    loader_version: Option<&str>,
    runtime_executable: &Path,
    version_root: &Path,
    install_status: &impl std::fmt::Debug,
    prepared_ms: u128,
    exit_status: ExitStatus,
) {
    println!("Using java executable: {}", runtime_executable.display());
    println!("version root: {}", version_root.display());
    println!("driver: {}", driver_name);
    println!("game version: {}", game_version);
    if let Some(loader_version) = loader_version {
        println!("loader version: {}", loader_version);
    }
    println!("install status: {:?}", install_status);
    println!("prepared in {}ms", prepared_ms);
    println!("process exited with: {}", exit_status);
}

fn collect_root_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

fn collect_recursive_native_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_recursive_native_files_into(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_recursive_native_files_into(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_recursive_native_files_into(&path, files)?;
            continue;
        }

        if is_native_binary(&path) {
            files.push(path);
        }
    }

    Ok(())
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

fn classpath_entries(arguments: &[String]) -> Option<Vec<String>> {
    let classpath_index = arguments
        .iter()
        .position(|argument| argument == "-cp" || argument == "-classpath")?;
    let classpath = arguments.get(classpath_index + 1)?;
    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };

    Some(classpath.split(separator).map(ToOwned::to_owned).collect())
}
