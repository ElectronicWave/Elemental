use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_core::auth::{
    accounts::Account, authorizers::offline::OfflineAuthorizer, credential::CredentialKind,
};
use elemental_launcher::AccountStore;
use elemental_shared::scope::Scope;

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elemental-account-store-{}-{nonce}",
        std::process::id()
    ))
}

#[tokio::test]
async fn account_store_persists_selects_and_removes_offline_account() {
    let root = temp_root();
    let scope = Scope::Directory(root.clone());
    let store = AccountStore::open(scope.clone()).await.unwrap();

    let account = store
        .sign_in(&OfflineAuthorizer {
            username: "TestPlayer".to_owned(),
        })
        .await
        .unwrap();
    let account_id = account.account_id();
    let expected_uuid = account.credential().await.unwrap().uuid;

    assert_eq!(account.display_name(), "TestPlayer");
    assert_eq!(store.list().await.unwrap().len(), 1);
    assert!(store.selected().await.unwrap().is_none());

    store.select(account_id).await.unwrap();

    let reopened = AccountStore::open(scope).await.unwrap();
    let selected = reopened.selected().await.unwrap().unwrap();
    assert_eq!(selected.account_id(), account_id);

    let account = reopened.account(account_id).await.unwrap();
    let credential = account.credential().await.unwrap();
    assert_eq!(credential.username, "TestPlayer");
    assert_eq!(credential.uuid, expected_uuid);
    assert_eq!(credential.kind, CredentialKind::Legacy);
    assert!(credential.access_token.is_empty());

    reopened.remove(account_id).await.unwrap();
    assert!(reopened.list().await.unwrap().is_empty());
    assert!(reopened.selected().await.unwrap().is_none());

    std::fs::remove_dir_all(root).unwrap();
}
