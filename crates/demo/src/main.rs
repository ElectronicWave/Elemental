//TODO REFACTOR ME WITH TUI/CLI

use elemental::model::mojang::MojangBaseUrl;
use elemental::online::mojang::MojangService;
use elemental::storage::game::GameStorage;

#[tokio::main]
async fn main() {
    // Test Download
    let service = MojangService::default();
    let launchmeta = service.launchmeta().await.unwrap();
    let pistonmeta = service
        .pistonmeta(
            launchmeta
                .versions
                .iter()
                .find(|data| data.id == "1.16.5")
                .unwrap()
                .url
                .clone(),
        )
        .await
        .unwrap();
    let storage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    storage.save_pistonmeta_data("1.16.5", &pistonmeta).unwrap();
    let objs = storage
        .get_and_save_objects_index(
            &service,
            pistonmeta.id.clone(),
            pistonmeta.asset_index.url.clone(),
        )
        .await
        .unwrap();
    let baseurl = MojangBaseUrl::default();

    println!("download objs");
    storage.download_objects("1.16.5", objs, &baseurl).unwrap();

    println!("download client");
    let _ = storage.download_client("1.16.5", &pistonmeta.downloads.client, &baseurl);
    println!("download libs");

    storage.download_libraries("1.16.5", &pistonmeta.libraries, &baseurl);
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
