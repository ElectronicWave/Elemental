use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_launcher::Launcher;

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

    let instances = tokio::spawn(async move { launcher.inspect_instances().await })
        .await
        .unwrap()
        .unwrap();

    assert!(instances.is_empty());
}
