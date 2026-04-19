use std::{
    path::PathBuf,
    process::ExitStatus,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use elemental_core::{
    auth::authorizers::offline::OfflineAuthorizer,
    install::ResolvedVanillaVersion,
    launcher::builder::LaunchBuilder,
    runtime::{distribution::Distribution, provider::all_providers},
    services::mojang::MojangService,
    storage::{game::GameStorage, layout::BaseLayout, version::VersionStorage},
};
use elemental_infra::downloader::{core::ElementalDownloader, report::SessionExecutionReport};

pub type DemoResolvedVersion = ResolvedVanillaVersion<BaseLayout, BaseLayout>;
pub type DemoVersionStorage = VersionStorage<BaseLayout, BaseLayout>;

#[derive(Debug, Clone)]
pub struct DemoInstallSpec {
    pub game_root: PathBuf,
    pub version_id: String,
    pub version_name: String,
}

#[derive(Debug, Clone)]
pub struct DemoLaunchSpec {
    pub username: String,
    pub runtime_version_prefix: String,
}

#[derive(Debug)]
pub struct DemoInstallSummary {
    pub version_root: PathBuf,
    pub download_reports: Vec<SessionExecutionReport>,
    pub download_elapsed: Duration,
    pub extract_elapsed: Duration,
}

#[derive(Debug)]
pub struct PreparedDemoVersion {
    pub resolved_version: DemoResolvedVersion,
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
    pub fn new(username: String, runtime_version_prefix: String) -> Self {
        Self {
            username,
            runtime_version_prefix,
        }
    }
}

impl DemoInstallSummary {
    pub fn render(&self) -> String {
        format!(
            "version root: {}\nreports: {:#?}\ndownload in {}ms\nextract in {}ms",
            self.version_root.display(),
            self.download_reports,
            self.download_elapsed.as_millis(),
            self.extract_elapsed.as_millis(),
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
    DemoLaunchSpec::new("IAMPlayer".to_owned(), "1.8".to_owned())
}

fn demo_downloader() -> Result<Arc<ElementalDownloader>> {
    ElementalDownloader::with_config_default().context("create demo downloader failed")
}

fn demo_service() -> MojangService {
    MojangService::default()
}

fn demo_storage(install_spec: &DemoInstallSpec) -> GameStorage<BaseLayout> {
    GameStorage::new(&install_spec.game_root, BaseLayout)
}

async fn resolve_demo_version(install_spec: &DemoInstallSpec) -> Result<DemoResolvedVersion> {
    let storage = demo_storage(install_spec);
    let service = demo_service();

    service
        .resolve_vanilla_version(
            &storage,
            install_spec.version_id.clone(),
            install_spec.version_name.clone(),
            BaseLayout,
        )
        .await
        .context("resolve demo version failed")
}

async fn download_demo_version(
    downloader: &Arc<ElementalDownloader>,
    resolved_version: &DemoResolvedVersion,
) -> Result<(Vec<SessionExecutionReport>, Duration)> {
    let planner = resolved_version.planner();
    let started_at = Instant::now();
    let download_reports = downloader
        .execute_planner(&planner)
        .await
        .context("download demo version artifacts failed")?;

    Ok((download_reports, started_at.elapsed()))
}

fn extract_demo_version(resolved_version: &DemoResolvedVersion) -> Result<Duration> {
    let started_at = Instant::now();
    resolved_version
        .version
        .extract_natives()
        .context("extract demo natives failed")?;

    Ok(started_at.elapsed())
}

pub async fn prepare_demo_version(install_spec: &DemoInstallSpec) -> Result<PreparedDemoVersion> {
    let downloader = demo_downloader()?;
    let resolved_version = resolve_demo_version(install_spec).await?;
    let (download_reports, download_elapsed) =
        download_demo_version(&downloader, &resolved_version).await?;
    let extract_elapsed = extract_demo_version(&resolved_version)?;
    let version_root = resolved_version.version.path.clone();

    Ok(PreparedDemoVersion {
        resolved_version,
        install_summary: DemoInstallSummary {
            version_root,
            download_reports,
            download_elapsed,
            extract_elapsed,
        },
    })
}

pub async fn find_runtime_distribution(runtime_version_prefix: &str) -> Result<Distribution> {
    Distribution::from_providers::<Vec<_>>(all_providers())
        .await
        .into_iter()
        .find(|distribution| {
            distribution
                .release
                .as_ref()
                .and_then(|release| release.jre_version.as_ref())
                .is_some_and(|version| version.starts_with(runtime_version_prefix))
        })
        .with_context(|| {
            format!(
                "can't find a java runtime with version prefix '{}'",
                runtime_version_prefix
            )
        })
}

fn offline_launch_builder(
    runtime: Distribution,
    version: DemoVersionStorage,
    launch_spec: &DemoLaunchSpec,
) -> LaunchBuilder<OfflineAuthorizer, BaseLayout, BaseLayout> {
    let authorizer = OfflineAuthorizer {
        username: launch_spec.username.clone(),
    };

    LaunchBuilder::new(authorizer, runtime, version).set_username(launch_spec.username.clone())
}

pub async fn launch_demo(
    install_spec: &DemoInstallSpec,
    launch_spec: &DemoLaunchSpec,
) -> Result<DemoLaunchSummary> {
    let runtime = find_runtime_distribution(&launch_spec.runtime_version_prefix).await?;
    let runtime_executable = runtime.executable();
    let prepared_demo = prepare_demo_version(install_spec).await?;
    let builder =
        offline_launch_builder(runtime, prepared_demo.resolved_version.version, launch_spec);
    let mut child = builder
        .launch()
        .await
        .context("launch demo process failed")?;
    let exit_status = child.wait().await.context("wait for demo process failed")?;

    Ok(DemoLaunchSummary {
        runtime_executable,
        install_summary: prepared_demo.install_summary,
        exit_status,
    })
}
