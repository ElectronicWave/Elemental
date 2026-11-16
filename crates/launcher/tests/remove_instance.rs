use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_launcher::{Launcher, RemoveInstanceOutcome};

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("elemental-remove-{}-{nonce}", std::process::id()))
}

#[tokio::test]
async fn remove_instance_removes_healthy_and_corrupt_instances() {
    let root = temp_root();
    fs::create_dir_all(root.join("versions/healthy")).unwrap();
    fs::write(root.join("versions/healthy/healthy.json"), "{}").unwrap();
    fs::create_dir_all(root.join("versions/broken")).unwrap();
    fs::write(root.join("versions/broken/broken.json"), "{").unwrap();
    let launcher = Launcher::builder().storage_root(root.clone()).build();

    assert_eq!(
        launcher.remove_instance("healthy").await.unwrap(),
        RemoveInstanceOutcome::Removed
    );
    assert!(!root.join("versions/healthy").exists());

    assert_eq!(
        launcher.remove_instance("broken").await.unwrap(),
        RemoveInstanceOutcome::Removed
    );
    assert!(!root.join("versions/broken").exists());

    assert_eq!(
        launcher.remove_instance("missing").await.unwrap(),
        RemoveInstanceOutcome::NotFound
    );
    assert!(launcher.remove_instance("../outside").await.is_err());

    fs::remove_dir_all(root).unwrap();
}
