use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Instant,
};

use anyhow::{Context, Result, bail};
use elemental::{
    core::{
        auth::authorizers::offline::OfflineAuthorizer,
        launcher::{command::LaunchCommand, process},
        storage::Storage,
    },
    driver::drivers::{
        fabric::{config::FabricLaunchConfig, driver::FabricDriver, source::FabricFlavor},
        vanilla::{config::VanillaLaunchConfig, driver::VanillaDriver},
        version_json::{
            BaseLayout, VersionJsonGameStorageExt, VersionJsonInstanceLayout,
            VersionJsonRootLayout, VersionJsonVersionStorageExt,
        },
    },
};

#[derive(Clone, Copy)]
enum DemoDriver {
    Vanilla,
    Fabric,
    LegacyFabric,
    Babric,
}

struct DemoConfig {
    driver: DemoDriver,
    storage_root: PathBuf,
    instance_name: String,
    game_version: String,
    loader_version: Option<String>,
}

struct VersionDiagnostics {
    version_root: PathBuf,
    metadata_path: PathBuf,
    version_jar_path: PathBuf,
    natives_root: PathBuf,
    natives_root_binaries: Vec<PathBuf>,
    natives_nested_binaries: Vec<PathBuf>,
    metadata_id: String,
    inherited_game_version: Option<String>,
    main_class: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = parse_demo_config(env::args().skip(1).collect())?;

    match config.driver {
        DemoDriver::Vanilla => run_vanilla_demo(config).await,
        DemoDriver::Fabric | DemoDriver::LegacyFabric | DemoDriver::Babric => {
            run_fabric_demo(config).await
        }
    }
}

fn parse_demo_config(arguments: Vec<String>) -> Result<DemoConfig> {
    let driver = arguments
        .first()
        .map(|value| parse_demo_driver(value.as_str()))
        .transpose()?
        .unwrap_or(DemoDriver::Fabric);

    match driver {
        DemoDriver::Vanilla => {
            let game_version = arguments
                .get(1)
                .cloned()
                .unwrap_or_else(|| "1.20.1".to_owned());
            let instance_name = arguments
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("MyVanilla-{game_version}"));

            Ok(DemoConfig {
                driver,
                storage_root: PathBuf::from(".minecraft"),
                instance_name,
                game_version,
                loader_version: None,
            })
        }
        DemoDriver::Fabric | DemoDriver::LegacyFabric | DemoDriver::Babric => {
            let game_version = arguments
                .get(1)
                .cloned()
                .unwrap_or_else(|| "1.20.1".to_owned());
            let loader_version = arguments
                .get(2)
                .cloned()
                .unwrap_or_else(|| "0.16.10".to_owned());
            let instance_name = arguments
                .get(3)
                .cloned()
                .unwrap_or_else(|| format!("MyFabric-{game_version}"));

            Ok(DemoConfig {
                driver,
                storage_root: PathBuf::from(".minecraft"),
                instance_name,
                game_version,
                loader_version: Some(loader_version),
            })
        }
    }
}

fn parse_demo_driver(value: &str) -> Result<DemoDriver> {
    match value {
        "vanilla" => Ok(DemoDriver::Vanilla),
        "fabric" => Ok(DemoDriver::Fabric),
        "legacyfabric" => Ok(DemoDriver::LegacyFabric),
        "babric" => Ok(DemoDriver::Babric),
        _ => bail!("unsupported demo driver: {value}"),
    }
}

impl DemoDriver {
    fn as_str(self) -> &'static str {
        match self {
            DemoDriver::Vanilla => "Vanilla",
            DemoDriver::Fabric => "Fabric",
            DemoDriver::LegacyFabric => "LegacyFabric",
            DemoDriver::Babric => "Babric",
        }
    }
}

async fn run_vanilla_demo(config: DemoConfig) -> Result<()> {
    let storage = Storage::new(config.storage_root.clone(), BaseLayout);
    let instance = storage
        .ensure_instance(config.instance_name.clone(), BaseLayout)
        .await?;
    let vanilla = VanillaDriver::with_defaults()?;
    let launch_config = VanillaLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let started_at = Instant::now();
    let prepared = vanilla
        .prepare(&instance, config.game_version.clone())
        .await?;
    let prepare_elapsed = started_at.elapsed();

    let diagnostics = collect_version_diagnostics(&prepared.resolved_version.version)?;
    let install_status = prepared.install_status.clone();
    let (runtime, command) = vanilla
        .build_launch_command(authorizer, &prepared, &launch_config)
        .await?;
    let runtime_executable = runtime.executable().to_path_buf();

    print_launch_diagnostics(
        config.driver.as_str(),
        None,
        &config.instance_name,
        &config.game_version,
        prepare_elapsed.as_millis(),
        &install_status,
        runtime_executable.as_path(),
        &diagnostics,
        &command,
    );

    let exit_status = run_logged_process(command).await?;
    print_summary(
        config.driver.as_str(),
        &config.game_version,
        None,
        runtime_executable.as_path(),
        diagnostics.version_root.as_path(),
        &install_status,
        prepare_elapsed.as_millis(),
        exit_status,
    );

    Ok(())
}

async fn run_fabric_demo(config: DemoConfig) -> Result<()> {
    let loader_version = config
        .loader_version
        .clone()
        .context("fabric demo requires a loader version")?;
    let storage = Storage::new(config.storage_root.clone(), BaseLayout);
    let instance = storage
        .ensure_instance(config.instance_name.clone(), BaseLayout)
        .await?;
    let fabric = FabricDriver::for_flavor(fabric_flavor(config.driver))?;
    let launch_config = FabricLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let started_at = Instant::now();
    let prepared = fabric
        .prepare(
            &instance,
            config.game_version.clone(),
            loader_version.clone(),
        )
        .await?;
    let prepare_elapsed = started_at.elapsed();

    let diagnostics = collect_version_diagnostics(&prepared.resolved_version.version)?;
    let install_status = prepared.install_status.clone();
    let (runtime, command) = fabric
        .build_launch_command(authorizer, &prepared, &launch_config)
        .await?;
    let runtime_executable = runtime.executable().to_path_buf();

    print_launch_diagnostics(
        config.driver.as_str(),
        Some(loader_version.as_str()),
        &config.instance_name,
        &config.game_version,
        prepare_elapsed.as_millis(),
        &install_status,
        runtime_executable.as_path(),
        &diagnostics,
        &command,
    );

    let exit_status = run_logged_process(command).await?;
    print_summary(
        config.driver.as_str(),
        &config.game_version,
        Some(loader_version.as_str()),
        runtime_executable.as_path(),
        diagnostics.version_root.as_path(),
        &install_status,
        prepare_elapsed.as_millis(),
        exit_status,
    );

    Ok(())
}

fn fabric_flavor(driver: DemoDriver) -> FabricFlavor {
    match driver {
        DemoDriver::Vanilla => FabricFlavor::Fabric,
        DemoDriver::Fabric => FabricFlavor::Fabric,
        DemoDriver::LegacyFabric => FabricFlavor::LegacyFabric,
        DemoDriver::Babric => FabricFlavor::Babric,
    }
}

fn collect_version_diagnostics<L, VL>(
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

async fn run_logged_process(command: LaunchCommand) -> Result<ExitStatus> {
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

fn print_launch_diagnostics(
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

fn print_summary(
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
