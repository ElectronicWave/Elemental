//TODO REFACTOR ME WITH TUI/CLI

use std::time::Duration;

use elemental::online::downloader::ElementalDownloader;
use elemental::online::mojang::MojangService;
use elemental::storage::game::GameStorage;

#[tokio::main]
async fn main() {
    // Test Download
    let service = MojangService::default();
    let version_name = "MyGame-1.16.5";
    let stroage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    stroage
        .download_version_all(&service, "1.16.5", version_name)
        .await
        .unwrap();
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            println!("{:?}", ElementalDownloader::shared().tracker.bps)
        }
    });
    ElementalDownloader::shared()
        .wait_group_tasks(version_name)
        .await;
    ElementalDownloader::shared().remove_task_group(version_name);
    stroage.extract_version_natives(version_name).unwrap();
}

#[tokio::test]
async fn test_game_run() {
    use elemental::bootstrap::java::JavaDistribution;
    let storage = GameStorage::new_ensure_dir("../../.minecraft").unwrap();
    let installs = JavaDistribution::get().await;
    let selected = installs
        .iter()
        .find(|e| e.install.path.contains("8"))
        .unwrap();

    let mut child = storage
        .launch_version(
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
