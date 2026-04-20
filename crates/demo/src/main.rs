use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result};
use elemental::{
    core::{
        auth::authorizers::offline::OfflineAuthorizer,
        launcher::{command::LaunchCommand, process},
        storage::Storage,
    },
    driver::drivers::{
        fabric::{config::FabricLaunchConfig, driver::FabricDriver},
        version_json::{BaseLayout, VersionJsonGameStorageExt, VersionJsonVersionStorageExt},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let storage_root = PathBuf::from(".minecraft");
    let instance_name = "MyFabric-1.20.1".to_owned();
    let game_version = "1.20.1".to_owned();
    let loader_version = "0.16.10".to_owned();
    let storage = Storage::new(storage_root, BaseLayout);
    let instance = storage
        .ensure_instance(instance_name.clone(), BaseLayout)
        .await?;
    let fabric = FabricDriver::with_defaults()?;
    let launch_config = FabricLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let started_at = Instant::now();
    let prepared = fabric
        .prepare(&instance, game_version.clone(), loader_version.clone())
        .await?;
    let prepare_elapsed = started_at.elapsed();

    let version = prepared.resolved_version.version.clone();
    let version_root = version.path.clone();
    let metadata = version.metadata()?;
    let metadata_path = version.metadata_path()?;
    let version_jar_path = version.jar_path()?;
    let natives_root = version.platform_natives_path();
    let natives_root_binaries = collect_root_files(&natives_root)?;
    let natives_nested_binaries = collect_recursive_native_files(&natives_root)?;
    let install_status = prepared.install_status.clone();

    let (runtime, command) = fabric
        .build_launch_command(authorizer, &prepared, &launch_config)
        .await?;
    let runtime_executable = runtime.executable().to_path_buf();

    print_launch_diagnostics(
        runtime_executable.as_path(),
        &version_root,
        metadata_path.as_path(),
        version_jar_path.as_path(),
        natives_root.as_path(),
        &natives_root_binaries,
        &natives_nested_binaries,
        &metadata.main_class,
        metadata.inherits_from.as_deref(),
        metadata.id.as_str(),
        command.args.len(),
        &command,
        &game_version,
        &loader_version,
        prepare_elapsed.as_millis(),
        &instance_name,
        &install_status,
    );

    let mut launched = process::spawn_command_logged(command)?;
    let mut lines = launched.lines;
    let log_task = tokio::spawn(async move {
        while let Some(line) = lines.recv().await {
            println!("[{:?}] {}", line.source, line.line);
        }
    });
    let exit_status = launched.child.wait().await?;
    log_task.await.context("join process log task failed")?;

    println!(
        "Using java executable: {}",
        runtime_executable.to_string_lossy()
    );
    println!("version root: {}", version_root.display());
    println!("driver: Fabric");
    println!("game version: {}", game_version);
    println!("loader version: {}", loader_version);
    println!("install status: {:?}", install_status);
    println!("prepared in {}ms", prepare_elapsed.as_millis());
    println!("process exited with: {}", exit_status);

    Ok(())
}

fn print_launch_diagnostics(
    runtime_executable: &Path,
    version_root: &Path,
    metadata_path: &Path,
    version_jar_path: &Path,
    natives_root: &Path,
    natives_root_binaries: &[PathBuf],
    natives_nested_binaries: &[PathBuf],
    main_class: &str,
    inherited_game_version: Option<&str>,
    metadata_id: &str,
    argument_count: usize,
    command: &LaunchCommand,
    game_version: &str,
    loader_version: &str,
    prepared_ms: u128,
    instance_name: &str,
    install_status: &impl std::fmt::Debug,
) {
    println!("instance: {}", instance_name);
    println!("driver: Fabric");
    println!("game version: {}", game_version);
    println!("loader version: {}", loader_version);
    println!("metadata id: {}", metadata_id);
    println!(
        "metadata inherits_from: {}",
        inherited_game_version.unwrap_or("<none>")
    );
    println!("metadata main class: {}", main_class);
    println!("Using java executable: {}", runtime_executable.display());
    println!("command executable: {}", command.program.display());
    println!(
        "command cwd: {}",
        command
            .cwd
            .as_ref()
            .map_or_else(|| "<none>".to_owned(), |cwd| cwd.display().to_string())
    );
    println!("command args count: {}", argument_count);
    println!("version root: {}", version_root.display());
    println!("metadata path: {}", metadata_path.display());
    println!("version jar: {}", version_jar_path.display());
    println!("natives root: {}", natives_root.display());
    println!("natives root exists: {}", natives_root.exists());
    println!("natives root file count: {}", natives_root_binaries.len());
    println!(
        "natives recursive binary count: {}",
        natives_nested_binaries.len()
    );
    println!(
        "natives root files: {}",
        format_path_list(natives_root_binaries, natives_root)
    );
    println!(
        "natives recursive binaries: {}",
        format_path_list(natives_nested_binaries, natives_root)
    );
    println!(
        "natives probe exists (lwjgl): {}",
        natives_root.join(expected_lwjgl_binary_name()).exists()
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
