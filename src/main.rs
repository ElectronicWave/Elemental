mod model;
mod online;
mod storage;
use model::mojang::MojangBaseUrl;
use online::mojang::MojangService;
use storage::game::GameStorage;

#[tokio::main]
async fn main() {
    let service = MojangService::default();
    let launchmeta = service.launchmeta().await.unwrap();
    let pistonmeta = service
        .pistonmeta(launchmeta.versions.first().unwrap().url.clone())
        .await
        .unwrap();

    let storage = GameStorage::new_ensure_dir(".minecraft").unwrap();
    let objs = storage
        .get_and_save_objects_index(
            &service,
            pistonmeta.id,
            pistonmeta.asset_index.url,
        )
        .await
        .unwrap();
   // storage.download_objects(objs, MojangBaseUrl::default(), None);
    //loop {}
}
