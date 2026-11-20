use elemental_core::auth::accounts::AccountId;
use elemental_launcher::{CapeStore, error::Error};
use elemental_shared::scope::Scope;

mod common;

const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xFC, 0xCF, 0xC0, 0x50,
    0x0F, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xA9, 0x8C, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[tokio::test]
async fn cape_store_imports_selects_and_removes() {
    let root = common::temp_root("elemental-cape-store");
    let scope = Scope::Directory(root.clone());
    let store = CapeStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store.import("Test", PNG.to_vec()).await.unwrap();
    let entries = store.list().await.unwrap();
    let hash = common::texture_hash(PNG);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].asset, asset);
    assert_eq!(asset.id.to_string(), hash);
    assert_eq!(
        entries[0].path,
        root.join(".elemental")
            .join("capes")
            .join(&hash[..2])
            .join(format!("{hash}.png"))
    );
    assert_eq!(std::fs::read(&entries[0].path).unwrap(), PNG);
    assert!(store.selected(account_id).await.unwrap().is_none());

    store.select(account_id, asset.id).await.unwrap();
    let selected = store.selected(account_id).await.unwrap().unwrap();
    assert_eq!(selected.asset, asset);
    assert!(selected.path.exists());

    store.remove(asset.id).await.unwrap();
    assert!(store.list().await.unwrap().is_empty());
    assert!(store.selected(account_id).await.unwrap().is_none());

    let reopened = CapeStore::open(scope).await.unwrap();
    assert!(reopened.list().await.unwrap().is_empty());
    assert!(reopened.selected(account_id).await.unwrap().is_none());

    common::remove(&root);
}

#[tokio::test]
async fn cape_store_dedupes_identical_content() {
    let root = common::temp_root("elemental-cape-store");
    let scope = Scope::Directory(root.clone());
    let store = CapeStore::open(scope).await.unwrap();

    let first = store.import("Mojang", PNG.to_vec()).await.unwrap();
    let second = store.import("Duplicate", PNG.to_vec()).await.unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.name, "Mojang");
    assert_eq!(store.list().await.unwrap().len(), 1);

    common::remove(&root);
}

#[tokio::test]
async fn cape_store_persists_imports_and_selections() {
    let root = common::temp_root("elemental-cape-store");
    let scope = Scope::Directory(root.clone());
    let store = CapeStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store.import("Mojang", PNG.to_vec()).await.unwrap();
    store.select(account_id, asset.id).await.unwrap();

    let reopened = CapeStore::open(scope).await.unwrap();
    let entries = reopened.list().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].asset, asset);

    let selected = reopened.selected(account_id).await.unwrap().unwrap();
    assert_eq!(selected.asset, asset);

    common::remove(&root);
}

#[tokio::test]
async fn cape_store_rejects_non_png_data() {
    let root = common::temp_root("elemental-cape-store");
    let scope = Scope::Directory(root.clone());
    let store = CapeStore::open(scope).await.unwrap();

    let error = store
        .import("Broken", b"not a png".to_vec())
        .await
        .unwrap_err();
    assert!(matches!(error, Error::CapeNotPng { .. }));
    assert!(store.list().await.unwrap().is_empty());

    common::remove(&root);
}

#[tokio::test]
async fn cape_store_renames_without_changing_identity() {
    let root = common::temp_root("elemental-cape-store");
    let scope = Scope::Directory(root.clone());
    let store = CapeStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store.import("Mojang", PNG.to_vec()).await.unwrap();
    store.select(account_id, asset.id).await.unwrap();

    store.rename(asset.id, "Migrator").await.unwrap();
    let entries = store.list().await.unwrap();
    assert_eq!(entries[0].asset.id, asset.id);
    assert_eq!(entries[0].asset.name, "Migrator");

    let selected = store.selected(account_id).await.unwrap().unwrap();
    assert_eq!(selected.asset.id, asset.id);
    assert_eq!(selected.asset.name, "Migrator");

    let reopened = CapeStore::open(scope).await.unwrap();
    let entries = reopened.list().await.unwrap();
    assert_eq!(entries[0].asset.name, "Migrator");

    let error = store.rename(asset.id, "\t").await.unwrap_err();
    assert!(matches!(error, Error::TextureNameEmpty));

    store.remove(asset.id).await.unwrap();
    let error = store.rename(asset.id, "Gone").await.unwrap_err();
    assert!(matches!(error, Error::CapeNotFound { .. }));

    common::remove(&root);
}
