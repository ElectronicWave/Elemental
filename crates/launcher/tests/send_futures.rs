#[cfg(feature = "microsoft-auth")]
use std::future::Future;

#[cfg(feature = "microsoft-auth")]
use elemental_core::auth::authorizers::microsoft::{
    MicrosoftAuthorizer, VerificationHandlerResult,
};
#[cfg(feature = "microsoft-auth")]
use elemental_launcher::AccountStore;
use elemental_launcher::Launcher;
#[cfg(feature = "microsoft-auth")]
use elemental_shared::scope::Scope;

mod common;

#[tokio::test]
async fn inspect_instances_future_is_send() {
    let launcher = Launcher::builder()
        .storage_root(common::temp_root("elemental-launcher-send"))
        .build();

    let report = tokio::spawn(async move { launcher.inspect_instances().await })
        .await
        .unwrap()
        .unwrap();

    assert!(report.instances.is_empty());
    assert!(report.failures.is_empty());
}

#[cfg(feature = "microsoft-auth")]
#[test]
fn microsoft_sign_in_future_is_send_and_static() {
    fn assert_send_static<F: Future + Send + 'static>(_: F) {}

    let handler: Box<dyn Fn(String, String) -> VerificationHandlerResult + Send + Sync> =
        Box::new(|_, _| Ok(()));
    let authorizer = MicrosoftAuthorizer {
        client_id: "client-id".to_owned(),
        verification_handler: handler,
    };
    let root = common::temp_root("elemental-launcher-send");
    let future = async move {
        let store = AccountStore::open(Scope::Directory(root)).await.unwrap();
        store.sign_in(&authorizer).await.unwrap();
    };

    assert_send_static(future);
}
