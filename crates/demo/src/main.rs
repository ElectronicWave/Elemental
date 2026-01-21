//TODO REFACTOR ME WITH TUI/CLI

use std::time::{Duration, SystemTime};

use elemental_core::services::downloader::ElementalDownloader;
use elemental_core::services::mojang::MojangService;
use elemental_core::legacystorage::game::GameStorage;

#[tokio::main]
async fn main() {
    // Test Download
    let downloader = ElementalDownloader::with_config_default().unwrap();
    let service = MojangService::default();
    let version_name = "MyGame-1.16.5";
    let stroage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    let s = SystemTime::now();
    let downloader_cloned = downloader.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;

            downloader_cloned
                .tracker
                .groups
                .get_async(version_name)
                .await
                .map(|state| println!("{:?}", state.bps));
        }
    });
    // if all file exists, it will cost 5-8s to vaildate sha1.
    stroage
        .download_version_all(&downloader, &service, "1.16.5", version_name)
        .await
        .unwrap();

    downloader.wait_group_empty(version_name).await;
    println!(
        "download in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
    downloader.remove_group(version_name).await;
    println!(
        "remove in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
    println!("start extract");

    stroage.extract_version_natives(version_name).unwrap();
    println!(
        "extract in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
}

#[tokio::test]
async fn test_game_run() {
    use elemental_core::runtime::{distribution::Distribution, provider::all_providers};

    let executable = Distribution::from_providers::<Vec<_>>(all_providers())
        .await
        .into_iter()
        .find(
            |e| match e.release.as_ref().and_then(|r| r.jre_version.as_ref()) {
                Some(v) => v.starts_with("1.8"),
                _ => false,
            },
        )
        .unwrap()
        .executable();

    println!("Using java executable: {}", executable.to_string_lossy());
    let storage = GameStorage::new_ensure_dir("../../.minecraft").unwrap();

    let mut child = storage
        .launch_version(
            "IAMPlayer",
            "MyGame-1.16.5",
            executable.to_string_lossy().to_string(),
            vec![
                "-Dfile.encoding=utf-8".to_owned(),
                "-Dsun.stdout.encoding=utf-8".to_owned(),
                "-Dsun.stderr.encoding=utf-8".to_owned(),
            ],
        )
        .unwrap();
    child.wait().await.unwrap();
}
