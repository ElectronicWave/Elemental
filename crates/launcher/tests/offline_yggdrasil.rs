#![cfg(feature = "yggdrasil-auth")]

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_core::auth::{
    accounts::Account,
    authorizers::yggdrasil::{YGGDRASIL_AUTHORIZER_ID, YggdrasilAuthorizer},
    credential::CredentialKind,
};
use elemental_launcher::AccountStore;
use elemental_shared::scope::Scope;

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elemental-local-yggdrasil-{}-{nonce}",
        std::process::id()
    ))
}

#[tokio::test]
async fn offline_yggdrasil_account_survives_restart_without_base_url() {
    let root = temp_root();
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

    std::fs::remove_dir_all(root).unwrap();
}
