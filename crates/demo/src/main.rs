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
    GameStorage::new_ensure_dir(".minecraft")
        .unwrap()
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
}

#[tokio::test]
async fn test_game_run() {
    use elemental::bootstrap::java::JavaDistribution;
    use elemental::model::launchenvs::LaunchEnvs;
    use elemental::model::mojang::PistonMetaData;
    use std::fs::File;

    let storage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    let file = File::open(storage.join("versions").join("1.16.5").join("1.16.5.json")).unwrap();
    let pistonmeta: PistonMetaData = serde_json::from_reader(file).unwrap();
    let launchenvs = LaunchEnvs::offline_player(
        "Elemental".to_owned(),
        storage.root.clone(),
        storage
            .join("versions")
            .join("1.16.5")
            .to_string_lossy()
            .to_string(),
        &pistonmeta,
    )
    .unwrap();

    let installs = JavaDistribution::get().await;
    let selected = installs
        .iter()
        .find(|e| e.install.path.contains("jdk-8"))
        .unwrap();
    let jvm = pistonmeta.arguments.get_jvm_arguments();
    let game = pistonmeta.arguments.get_game_arguments();

    let mut launchargs = vec![];
    //TODO Launcher extra arg here.
    launchargs.extend(vec![
        "-Dfile.encoding=utf-8".to_owned(),
        "-Dsun.stdout.encoding=utf-8".to_owned(),
        "-Dsun.stderr.encoding=utf-8".to_owned(),
    ]);

    launchargs.extend(launchenvs.apply_launchenvs(jvm).unwrap());
    launchargs.push(pistonmeta.main_class.clone());
    launchargs.extend(launchenvs.apply_launchenvs(game).unwrap());
    let mut cmd = std::process::Command::new(&selected.install.path); // FIXME NOT A EXECUTABLE
    cmd.args(launchargs);
    let out = cmd.output().unwrap();
    println!("{}", String::from_utf8(out.stderr).unwrap());
    println!("{}", String::from_utf8(out.stdout).unwrap());
}
