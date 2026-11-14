use std::{
    future::Future,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_core::auth::authorizers::microsoft::{
    MicrosoftAuthorizer, VerificationHandlerResult,
};
use elemental_launcher::{AccountStore, Launcher};
use elemental_shared::scope::Scope;

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elemental-launcher-send-{}-{nonce}",
        std::process::id()
    ))
}

#[tokio::test]
async fn inspect_instances_future_is_send() {
    let launcher = Launcher::builder().storage_root(temp_root()).build();

    let report = tokio::spawn(async move { launcher.inspect_instances().await })
        .await
        .unwrap()
        .unwrap();

    assert!(report.instances.is_empty());
    assert!(report.failures.is_empty());
}

#[test]
fn microsoft_sign_in_future_is_send_and_static() {
    fn assert_send_static<F: Future + Send + 'static>(_: F) {}

    let handler: Box<dyn Fn(String, String) -> VerificationHandlerResult + Send + Sync> =
        Box::new(|_, _| Ok(()));
    let authorizer = MicrosoftAuthorizer {
        client_id: "client-id".to_owned(),
        verification_handler: handler,
    };
    let root = temp_root();
    let future = async move {
        let store = AccountStore::open(Scope::Directory(root)).await.unwrap();
        store.sign_in(&authorizer).await.unwrap();
    };

    assert_send_static(future);
}
