#![allow(dead_code)]
mod bootstrap;
mod error;
mod model;
mod offline;
mod online;
mod storage;

use futures::future::join_all;
use model::mojang::MojangBaseUrl;
use online::mojang::MojangService;
use storage::game::GameStorage;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

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

    let token = CancellationToken::new();
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
    join_all(storage.download_objects(objs, &baseurl, &token).unwrap()).await;
    println!("download client");
    storage
        .download_client("1.16.5", &pistonmeta.downloads.client, &baseurl, &token)
        .unwrap()
        .await
        .unwrap();
    println!("download libs");
    join_all(
        storage
            .download_libraries(&pistonmeta.libraries, "1.16.5", &baseurl, &token)
            .into_iter()
            .filter_map(|e| {
                if let Ok(Some(handle)) = e {
                    Some(handle)
                } else {
                    None
                }
            })
            .collect::<Vec<JoinHandle<()>>>(),
    )
    .await;
}

#[test]
fn test_game_run() {
    use bootstrap::java::JavaInstall;
    use model::launchenvs::LaunchEnvs;
    use model::mojang::PistonMetaData;
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

    let installs = JavaInstall::get_all_java_distribution();
    let selected = installs.iter().find(|e| e.path.contains("jdk-8")).unwrap();
    let jvm = pistonmeta.arguments.get_jvm_arguments();
    let game = pistonmeta.arguments.get_game_arguments();

    //TODO Apply launchenvs to args

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
    let mut cmd = std::process::Command::new(selected.get_executable_file_path().unwrap());
    cmd.args(launchargs);
    let out = cmd.output().unwrap();
    println!("{}", String::from_utf8(out.stderr).unwrap());
    println!("{}", String::from_utf8(out.stdout).unwrap());
}
