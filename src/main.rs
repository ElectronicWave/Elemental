#![allow(dead_code)]
mod model;
mod online;
mod storage;
use futures::future::join_all;
use model::mojang::MojangBaseUrl;
use online::mojang::MojangService;
use storage::game::GameStorage;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
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
    let token = CancellationToken::new();
    let objs = storage
        .get_and_save_objects_index(&service, pistonmeta.id, pistonmeta.asset_index.url)
        .await
        .unwrap();
    let baseurl = MojangBaseUrl::default();
    println!("download objs");
    join_all(
        storage
            .download_objects(objs, &baseurl, token.clone())
            .unwrap(),
    )
    .await;
    println!("download client");
    storage
        .download_client(
            "1.16.5".to_owned(),
            pistonmeta.downloads.client,
            &baseurl,
            token,
       
        )
        .unwrap()
        .await
        .unwrap();
    // println!("download libs");
}
