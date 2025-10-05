//TODO REFACTOR ME WITH TUI/CLI

use std::time::{Duration, SystemTime};

use elemental_core::online::downloader::ElementalDownloader;
use elemental_core::online::mojang::MojangService;
use elemental_core::storage::game::GameStorage;

#[tokio::main]
async fn main() {
    // Test Download
    let downloader = ElementalDownloader::new();
    let service = MojangService::default();
    let version_name = "MyGame-1.16.5";
    let stroage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    let s = SystemTime::now();
    let downloader_cloned = downloader.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            println!("{:?}", downloader_cloned.tracker.bps);
        }
    });
    // if all file exists, it will cost 5-8s to vaildate sha1.
    stroage
        .download_version_all(&downloader, &service, "1.16.5", version_name)
        .await
        .unwrap();

    downloader.wait_group_tasks(version_name).await;
    println!(
        "costs {}ms",
        SystemTime::now().duration_since(s).unwrap().as_millis()
    );
    downloader.remove_task_group(version_name).await;
    println!("{:?}", downloader.tracker.bps);
    println!("start extract");

    stroage.extract_version_natives(version_name).unwrap();
}

#[tokio::test]
async fn test_game_run() {
    use elemental_core::bootstrap::java::JavaDistribution;
    let storage = GameStorage::new_ensure_dir("../../.minecraft").unwrap();
    let installs = JavaDistribution::get().await;
    let selected = installs
        .iter()
        .find(|e| e.install.path.contains("8"))
        .unwrap();

    let mut child = storage
        .launch_version(
            "IAMPlayer",
            "MyGame-1.16.5",
            format!("{}/java.exe", selected.install.path),
            vec![
                "-Dfile.encoding=utf-8".to_owned(),
                "-Dsun.stdout.encoding=utf-8".to_owned(),
                "-Dsun.stderr.encoding=utf-8".to_owned(),
            ],
        )
        .unwrap();
    child.wait().await.unwrap();
}
