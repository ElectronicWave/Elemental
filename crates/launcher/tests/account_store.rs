use elemental_core::auth::{
    accounts::{Account, AccountId},
    authorizers::offline::OfflineAuthorizer,
    credential::CredentialKind,
};
use elemental_launcher::{AccountRef, AccountStore};
use elemental_shared::scope::Scope;

mod common;

#[tokio::test]
async fn account_store_persists_selects_and_removes_offline_account() {
    let root = common::temp_root("elemental-account-store");
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

    common::remove(&root);
}

#[tokio::test]
async fn account_store_skips_unknown_authorizer_entries() {
    let root = common::temp_root("elemental-account-store-unknown-authorizer");
    let scope = Scope::Directory(root.clone());
    let store = AccountStore::open(scope.clone()).await.unwrap();

    store
        .sign_in(&OfflineAuthorizer {
            username: "TestPlayer".to_owned(),
        })
        .await
        .unwrap();

    let unsupported = AccountRef {
        account_id: AccountId::new(),
        authorizer: String::from("particle-yggdrasil-local"),
    };
    let index_path = root.join(".elemental").join("accounts").join("index.json");
    let mut index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap()).unwrap();
    let unsupported_value = serde_json::to_value(&unsupported).unwrap();
    let value = index.get_mut("value").unwrap();
    value["accounts"]
        .as_array_mut()
        .unwrap()
        .push(unsupported_value.clone());
    value["selected"] = unsupported_value;
    std::fs::write(&index_path, serde_json::to_string(&index).unwrap()).unwrap();

    let reopened = AccountStore::open(scope).await.unwrap();

    let listing = reopened.try_list().await.unwrap();
    assert_eq!(listing.accounts.len(), 1);
    assert_eq!(listing.unsupported, vec![unsupported]);
    assert_eq!(reopened.list().await.unwrap().len(), 1);
    assert!(reopened.selected().await.unwrap().is_none());

    common::remove(&root);
}
