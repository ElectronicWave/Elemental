use std::{
    path::PathBuf,
    process::ExitStatus,
    time::{Duration, Instant},
};

use anyhow::Result;
use elemental_core::{
    auth::authorizers::offline::OfflineAuthorizer,
    install::{ReadyVanillaVersion, VanillaInstallStatus},
    launcher::vanilla::{VanillaLaunchOptions, VanillaLauncher, VanillaVersionSpec},
    storage::{game::GameStorage, layout::BaseLayout},
};

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
    pub ready_elapsed: Duration,
}

#[derive(Debug)]
pub struct ReadyDemoVersion {
    pub ready_version: ReadyVanillaVersion<BaseLayout, BaseLayout>,
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
            "version root: {}\ninstall status: {:?}\nready in {}ms",
            self.version_root.display(),
            self.install_status,
            self.ready_elapsed.as_millis(),
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

fn demo_launcher() -> Result<VanillaLauncher> {
    VanillaLauncher::with_defaults()
}

fn demo_storage(install_spec: &DemoInstallSpec) -> GameStorage<BaseLayout> {
    GameStorage::new(&install_spec.game_root, BaseLayout)
}

fn demo_version_spec(install_spec: &DemoInstallSpec) -> VanillaVersionSpec<BaseLayout> {
    VanillaVersionSpec::new(
        install_spec.version_id.clone(),
        install_spec.version_name.clone(),
        BaseLayout,
    )
}

pub async fn ready_demo_version(install_spec: &DemoInstallSpec) -> Result<ReadyDemoVersion> {
    let launcher = demo_launcher()?;
    let storage = demo_storage(install_spec);
    let started_at = Instant::now();
    let ready_version = launcher
        .ready(&storage, &demo_version_spec(install_spec))
        .await?;
    let ready_elapsed = started_at.elapsed();
    let version_root = ready_version.resolved_version.version.path.clone();
    let install_status = ready_version.install_status;

    Ok(ReadyDemoVersion {
        ready_version,
        install_summary: DemoInstallSummary {
            version_root,
            install_status,
            ready_elapsed,
        },
    })
}

pub async fn launch_demo(
    install_spec: &DemoInstallSpec,
    launch_spec: &DemoLaunchSpec,
) -> Result<DemoLaunchSummary> {
    let launcher = demo_launcher()?;
    let ready_demo = ready_demo_version(install_spec).await?;
    let launch_options = VanillaLaunchOptions::new(demo_version_spec(install_spec));
    let authorizer = OfflineAuthorizer {
        username: launch_spec.username.clone(),
    };
    let launched = launcher
        .launch_ready(ready_demo.ready_version, &launch_options, authorizer)
        .await?;
    let runtime_executable = launched.runtime.executable();
    let mut child = launched.child;
    let exit_status = child.wait().await?;

    Ok(DemoLaunchSummary {
        runtime_executable,
        install_summary: ready_demo.install_summary,
        exit_status,
    })
}
