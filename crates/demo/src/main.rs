//TODO REFACTOR ME WITH TUI/CLI

use std::time::{Duration, SystemTime};

use elemental_core::services::mojang::MojangService;
use elemental_core::storage::{game::GameStorage, layout::BaseLayout};
use elemental_infra::downloader::core::ElementalDownloader;

#[tokio::main]
async fn main() {
    // Test Download
    let downloader = ElementalDownloader::with_config_default().unwrap();
    let service = MojangService::default();
    let version_name = "MyGame-1.16.5";
    let storage = GameStorage::new(".minecraft", BaseLayout);
    let resolved = service
        .resolve_vanilla_version(&storage, "1.16.5", version_name, BaseLayout)
        .await
        .unwrap();
    let planner = resolved.planner();
    let s = SystemTime::now();
    let reports = downloader.execute_planner(&planner).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    println!("reports: {reports:#?}");
    println!(
        "download in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
    println!("start extract");

    resolved.version.extract_natives().unwrap();
    println!(
        "extract in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
}

#[tokio::test]
async fn test_game_run() {
    use elemental_core::{
        auth::authorizers::offline::OfflineAuthorizer,
        launcher::builder::LaunchBuilder,
        runtime::{distribution::Distribution, provider::all_providers},
    };

    let executable = Distribution::from_providers::<Vec<_>>(all_providers())
        .await
        .into_iter()
        .find(
            |e| match e.release.as_ref().and_then(|r| r.jre_version.as_ref()) {
                Some(v) => v.starts_with("1.8"),
                _ => false,
            },
        )
        .unwrap();

    println!(
        "Using java executable: {}",
        executable.executable().to_string_lossy()
    );
    let storage = GameStorage::new("../../.minecraft", BaseLayout);
    let service = MojangService::default();
    let resolved = service
        .resolve_vanilla_version(&storage, "1.16.5", "MyGame-1.16.5", BaseLayout)
        .await
        .unwrap();
    let downloader = ElementalDownloader::with_config_default().unwrap();
    let planner = resolved.planner();
    downloader.execute_planner(&planner).await.unwrap();
    resolved.version.extract_natives().unwrap();

    let authorizer = OfflineAuthorizer {
        username: "IAMPlayer".to_owned(),
    };
    let builder = LaunchBuilder::new(authorizer, executable, resolved.version)
        .set_username("IAMPlayer".to_owned());

    let mut child = builder.launch().await.unwrap();
    child.wait().await.unwrap();
}
