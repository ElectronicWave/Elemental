use std::fs;

use elemental_launcher::{Launcher, RemoveInstanceOutcome};

mod common;

#[tokio::test]
async fn remove_instance_removes_healthy_and_corrupt_instances() {
    let root = common::temp_root("elemental-remove");
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

    common::remove(&root);
}
