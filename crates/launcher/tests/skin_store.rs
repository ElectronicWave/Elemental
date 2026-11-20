use elemental_core::auth::accounts::AccountId;
use elemental_launcher::{SkinModel, SkinStore, error::Error};
use elemental_shared::scope::Scope;
use sha2::{Digest, Sha256};

mod common;

const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xFC, 0xCF, 0xC0, 0x50,
    0x0F, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xA9, 0x8C, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn png_hash() -> String {
    hex::encode(Sha256::digest(PNG))
}

#[tokio::test]
async fn skin_store_imports_selects_and_removes() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store
        .import("Test", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    let entries = store.list().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].asset, asset);
    assert_eq!(asset.id.to_string(), png_hash());
    assert_eq!(
        entries[0].path,
        root.join(".elemental")
            .join("skins")
            .join(&png_hash()[..2])
            .join(format!("{}.png", png_hash()))
    );
    assert_eq!(std::fs::read(&entries[0].path).unwrap(), PNG);
    assert!(store.selected(account_id).await.unwrap().is_none());

    store
        .select(account_id, asset.id, SkinModel::Slim)
        .await
        .unwrap();
    let selected = store.selected(account_id).await.unwrap().unwrap();
    assert_eq!(selected.asset.id, asset.id);
    assert_eq!(selected.model, SkinModel::Slim);
    assert!(selected.path.exists());

    store.remove(asset.id).await.unwrap();
    assert!(store.list().await.unwrap().is_empty());
    assert!(store.selected(account_id).await.unwrap().is_none());

    let reopened = SkinStore::open(scope).await.unwrap();
    assert!(reopened.list().await.unwrap().is_empty());
    assert!(reopened.selected(account_id).await.unwrap().is_none());

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_dedupes_identical_content() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope.clone()).await.unwrap();

    let first = store
        .import("Steve", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    let second = store
        .import("Duplicate", PNG.to_vec(), SkinModel::Slim)
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.name, "Steve");
    assert_eq!(second.default_model, SkinModel::Classic);
    assert_eq!(store.list().await.unwrap().len(), 1);

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_updates_model_without_changing_content() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store
        .import("Steve", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    let path = store.list().await.unwrap()[0].path.clone();
    store
        .select(account_id, asset.id, SkinModel::Classic)
        .await
        .unwrap();

    store.set_model(asset.id, SkinModel::Slim).await.unwrap();

    let reopened = SkinStore::open(scope).await.unwrap();
    let entry = &reopened.list().await.unwrap()[0];
    assert_eq!(entry.asset.id, asset.id);
    assert_eq!(entry.asset.default_model, SkinModel::Slim);
    assert_eq!(entry.path, path);
    assert_eq!(std::fs::read(&entry.path).unwrap(), PNG);
    assert_eq!(
        reopened.selected(account_id).await.unwrap().unwrap().model,
        SkinModel::Classic
    );

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_repairs_corrupt_content() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope).await.unwrap();

    let asset = store
        .import("Steve", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    let path = store.list().await.unwrap()[0].path.clone();
    std::fs::write(&path, b"corrupt").unwrap();

    let imported = store
        .import("Steve", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    assert_eq!(imported.id, asset.id);
    assert_eq!(std::fs::read(&path).unwrap(), PNG);
    assert_eq!(store.list().await.unwrap().len(), 1);

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_gc_removes_unreferenced_content() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope.clone()).await.unwrap();

    store
        .import("Steve", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    let referenced = store.list().await.unwrap()[0].path.clone();
    drop(store);

    let orphan_dir = root.join(".elemental").join("skins").join("aa");
    std::fs::create_dir_all(&orphan_dir).unwrap();
    let orphan = orphan_dir.join(format!("{}.png", "a".repeat(64)));
    std::fs::write(&orphan, b"orphan").unwrap();

    SkinStore::open(scope).await.unwrap();

    assert!(!orphan.exists());
    assert!(referenced.exists());

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_persists_imports_and_selections() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store
        .import("Steve", PNG.to_vec(), SkinModel::Slim)
        .await
        .unwrap();
    store
        .select(account_id, asset.id, SkinModel::Slim)
        .await
        .unwrap();

    let reopened = SkinStore::open(scope).await.unwrap();
    let entries = reopened.list().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].asset, asset);

    let selected = reopened.selected(account_id).await.unwrap().unwrap();
    assert_eq!(selected.asset, asset);
    assert_eq!(selected.model, SkinModel::Slim);

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_rejects_non_png_data() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope).await.unwrap();

    let error = store
        .import("Broken", b"not a png".to_vec(), SkinModel::Classic)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        elemental_launcher::error::Error::SkinNotPng { .. }
    ));
    assert!(store.list().await.unwrap().is_empty());

    common::remove(&root);
}

#[tokio::test]
async fn skin_store_renames_without_changing_identity() {
    let root = common::temp_root("elemental-skin-store");
    let scope = Scope::Directory(root.clone());
    let store = SkinStore::open(scope.clone()).await.unwrap();
    let account_id = AccountId::new();

    let asset = store
        .import("Steve", PNG.to_vec(), SkinModel::Classic)
        .await
        .unwrap();
    store
        .select(account_id, asset.id, SkinModel::Classic)
        .await
        .unwrap();

    store.rename(asset.id, "Alex").await.unwrap();
    let entries = store.list().await.unwrap();
    assert_eq!(entries[0].asset.id, asset.id);
    assert_eq!(entries[0].asset.name, "Alex");

    let selected = store.selected(account_id).await.unwrap().unwrap();
    assert_eq!(selected.asset.id, asset.id);
    assert_eq!(selected.asset.name, "Alex");

    let reopened = SkinStore::open(scope).await.unwrap();
    let entries = reopened.list().await.unwrap();
    assert_eq!(entries[0].asset.name, "Alex");

    let error = store.rename(asset.id, "   ").await.unwrap_err();
    assert!(matches!(error, Error::TextureNameEmpty));

    store.remove(asset.id).await.unwrap();
    let error = store.rename(asset.id, "Gone").await.unwrap_err();
    assert!(matches!(error, Error::SkinNotFound { .. }));

    common::remove(&root);
}
