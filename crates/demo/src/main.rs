//TODO REFACTOR ME WITH TUI/CLI

use std::time::{Duration, SystemTime};

use elemental_core::legacystorage::game::GameStorage;
use elemental_core::services::mojang::MojangService;
use elemental_infra::downloader::core::ElementalDownloader;

#[tokio::main]
async fn main() {
    // Test Download
    let downloader = ElementalDownloader::with_config_default().unwrap();
    let service = MojangService::default();
    let version_name = "MyGame-1.16.5";
    let storage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    let s = SystemTime::now();
    let session = storage
        .download_version_all(&downloader, &service, "1.16.5", version_name)
        .await
        .unwrap();
    let session_id = session.id();
    let downloader_cloned = downloader.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;

            downloader_cloned
                .tracker
                .sessions
                .get_async(&session_id)
                .await
                .map(|state| println!("{:?}", state.bps));
        }
    });
    // if all file exists, it will cost 5-8s to vaildate sha1.
    session.wait_empty().await.unwrap();
    println!(
        "download in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
    session.remove().await.unwrap();
    println!(
        "remove in {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
    println!("start extract");

    storage.extract_version_natives(version_name).unwrap();
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
