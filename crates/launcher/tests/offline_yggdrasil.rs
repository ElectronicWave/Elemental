#![cfg(feature = "yggdrasil-auth")]

use elemental_core::auth::{
    accounts::Account,
    authorizers::yggdrasil::{YGGDRASIL_AUTHORIZER_ID, YggdrasilAuthorizer},
    credential::CredentialKind,
};
use elemental_launcher::AccountStore;
use elemental_shared::scope::Scope;

mod common;

#[tokio::test]
async fn offline_yggdrasil_account_survives_restart_without_base_url() {
    let root = common::temp_root("elemental-local-yggdrasil");
    let scope = Scope::Directory(root.clone());
    let store = AccountStore::open(scope.clone()).await.unwrap();

    let account = store
        .sign_in(&YggdrasilAuthorizer {
            base_url: None,
            username: "TestPlayer".to_owned(),
            password: String::new(),
            client_token: None,
        })
        .await
        .unwrap();
    let account_id = account.account_id();
    let expected_uuid = account.credential().await.unwrap().uuid;

    assert_eq!(account.authorizer_id(), YGGDRASIL_AUTHORIZER_ID);

    let reopened = AccountStore::open(scope).await.unwrap();
    let account = reopened.account(account_id).await.unwrap();
    let credential = account.credential().await.unwrap();

    assert_eq!(account.authorizer_id(), YGGDRASIL_AUTHORIZER_ID);
    assert_eq!(credential.username, "TestPlayer");
    assert_eq!(credential.uuid, expected_uuid);
    assert_eq!(credential.kind, CredentialKind::Legacy);
    assert!(credential.auth_server.is_none());

    common::remove(&root);
}
