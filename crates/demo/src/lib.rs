use std::{
    path::PathBuf,
    process::ExitStatus,
    time::{Duration, Instant},
};

use anyhow::Result;
use elemental_core::{auth::authorizers::offline::OfflineAuthorizer, storage::Storage};
use elemental_driver::vanilla::{
    PreparedVanillaVersion, VanillaDriver, VanillaInstallStatus, VanillaLaunchConfig,
};
use elemental_driver::version_json::BaseLayout;

#[derive(Debug, Clone)]
pub struct DemoInstallSpec {
    pub game_root: PathBuf,
    pub version_id: String,
    pub version_name: String,
}

#[derive(Debug, Clone)]
pub struct DemoLaunchSpec {
    pub username: String,
}

#[derive(Debug)]
pub struct DemoInstallSummary {
    pub version_root: PathBuf,
    pub install_status: VanillaInstallStatus,
    pub prepare_elapsed: Duration,
}

#[derive(Debug)]
pub struct PreparedDemoVersion {
    pub prepared_version: PreparedVanillaVersion<BaseLayout, BaseLayout>,
    pub install_summary: DemoInstallSummary,
}

#[derive(Debug)]
pub struct DemoLaunchSummary {
    pub runtime_executable: PathBuf,
    pub install_summary: DemoInstallSummary,
    pub exit_status: ExitStatus,
}

impl DemoInstallSpec {
    pub fn new(game_root: PathBuf, version_id: String, version_name: String) -> Self {
        Self {
            game_root,
            version_id,
            version_name,
        }
    }
}

impl DemoLaunchSpec {
    pub fn new(username: String) -> Self {
        Self { username }
    }
}

impl DemoInstallSummary {
    pub fn render(&self) -> String {
        format!(
            "version root: {}\ninstall status: {:?}\nprepared in {}ms",
            self.version_root.display(),
            self.install_status,
            self.prepare_elapsed.as_millis(),
        )
    }
}

pub fn demo_install_spec() -> DemoInstallSpec {
    DemoInstallSpec::new(
        PathBuf::from(".minecraft"),
        "1.16.5".to_owned(),
        "MyGame-1.16.5".to_owned(),
    )
}

pub fn demo_launch_spec() -> DemoLaunchSpec {
    DemoLaunchSpec::new("IAMPlayer".to_owned())
}

fn demo_launcher() -> Result<VanillaDriver> {
    VanillaDriver::with_defaults()
}

fn demo_storage(install_spec: &DemoInstallSpec) -> Storage<BaseLayout> {
    Storage::new(&install_spec.game_root, BaseLayout)
}

pub async fn prepare_demo_version(install_spec: &DemoInstallSpec) -> Result<PreparedDemoVersion> {
    let launcher = demo_launcher()?;
    let storage = demo_storage(install_spec);
    let started_at = Instant::now();
    let prepared_version = launcher
        .prepare(
            &storage,
            install_spec.version_id.clone(),
            install_spec.version_name.clone(),
            BaseLayout,
        )
        .await?;
    let prepare_elapsed = started_at.elapsed();
    let version_root = prepared_version.resolved_version.version.path.clone();
    let install_status = prepared_version.install_status;

    Ok(PreparedDemoVersion {
        prepared_version,
        install_summary: DemoInstallSummary {
            version_root,
            install_status,
            prepare_elapsed,
        },
    })
}

pub async fn launch_demo(
    install_spec: &DemoInstallSpec,
    launch_spec: &DemoLaunchSpec,
) -> Result<DemoLaunchSummary> {
    let launcher = demo_launcher()?;
    let prepared_demo = prepare_demo_version(install_spec).await?;
    let launch_config = VanillaLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: launch_spec.username.clone(),
    };
    let launched = launcher
        .launch(prepared_demo.prepared_version, &launch_config, authorizer)
        .await?;
    let runtime_executable = launched.runtime.executable();
    let mut child = launched.child;
    let exit_status = child.wait().await?;

    Ok(DemoLaunchSummary {
        runtime_executable,
        install_summary: prepared_demo.install_summary,
        exit_status,
    })
}
