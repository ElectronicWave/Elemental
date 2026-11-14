use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use elemental_launcher::Launcher;

const VALID_METADATA: &str = r#"{
  "assetIndex": {
    "id": "test",
    "sha1": "",
    "size": 0,
    "totalSize": 0,
    "url": "https://example.invalid/assets.json"
  },
  "assets": "test",
  "complianceLevel": 0,
  "downloads": {
    "client": {
      "sha1": "",
      "size": 0,
      "url": "https://example.invalid/client.jar"
    }
  },
  "id": "valid",
  "javaVersion": {
    "component": "jre",
    "majorVersion": 17
  },
  "libraries": [],
  "mainClass": "net.minecraft.client.main.Main",
  "minimumLauncherVersion": 0,
  "releaseTime": "2024-01-01T00:00:00Z",
  "time": "2024-01-01T00:00:00Z",
  "type": "release"
}"#;

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("elemental-inspect-{}-{nonce}", std::process::id()))
}

fn write_instance(root: &PathBuf, name: &str, metadata: &str) {
    let directory = root.join("versions").join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(format!("{name}.json")), metadata).unwrap();
    fs::write(directory.join(format!("{name}.jar")), []).unwrap();
}

#[tokio::test]
async fn inspect_instances_keeps_healthy_instances_when_one_is_corrupt() {
    let root = temp_root();
    write_instance(&root, "broken", "{");
    write_instance(&root, "healthy", VALID_METADATA);
    let launcher = Launcher::builder().storage_root(root.clone()).build();

    let report = launcher.inspect_instances().await.unwrap();

    assert_eq!(report.instances.len(), 1);
    assert_eq!(report.instances[0].instance_name, "healthy");
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].instance_name, "broken");
    assert_eq!(
        report.failures[0].path,
        root.join("versions").join("broken")
    );

    fs::remove_dir_all(root).unwrap();
}
