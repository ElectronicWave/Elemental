#[cfg(feature = "driver-all")]
use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
#[cfg(feature = "driver-all")]
use elemental_driver::descriptors::{ALL_DRIVERS, from_id};
use elemental_driver::{
    descriptors::VANILLA_DRIVER,
    driver::{Driver, DriverDescriptor, InstalledDriver, PrepareRequest},
    drivers::vanilla::{catalog::VanillaCatalog, driver::VanillaDriver},
    error::Result,
    families::version_json::{InstanceStorage, PreparedVersionJsonInstance},
    inspect::InstanceProbe,
};
use elemental_launcher::{DriverCatalog, DriverRegistration, Launcher};

mod common;

const CUSTOM: DriverDescriptor = DriverDescriptor {
    id: "third-party",
    name: "Third Party",
};

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
  "id": "custom",
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

struct DetectsEverything;

#[async_trait]
impl Driver for DetectsEverything {
    fn descriptor(&self) -> DriverDescriptor {
        CUSTOM
    }

    async fn inspect(&self, _probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        Ok(Some(InstalledDriver {
            driver: CUSTOM,
            driver_version: Some("1".to_string()),
            game_version: None,
            description: None,
        }))
    }

    async fn prepare(&self, _request: &PrepareRequest<'_>) -> Result<PreparedVersionJsonInstance> {
        unimplemented!("inspection test only")
    }

    async fn load_prepared(
        &self,
        _instance: &InstanceStorage,
    ) -> Result<PreparedVersionJsonInstance> {
        unimplemented!("inspection test only")
    }
}

#[cfg(feature = "driver-all")]
#[test]
fn registry_descriptors_follow_detection_order() {
    let launcher = Launcher::builder().build();
    let registered = launcher.drivers().descriptors().collect::<Vec<_>>();

    assert_eq!(registered, ALL_DRIVERS);
}

#[cfg(feature = "driver-all")]
#[test]
fn registry_descriptors_cover_all_built_in_descriptors() {
    let launcher = Launcher::builder().build();
    let supported: HashSet<&str> = launcher
        .drivers()
        .descriptors()
        .map(|driver| driver.id)
        .collect();
    let built_in: HashSet<&str> = ALL_DRIVERS.iter().map(|driver| driver.id).collect();

    assert_eq!(supported, built_in);

    for driver in ALL_DRIVERS {
        assert_eq!(from_id(driver.id), Some(*driver));
    }
}

#[test]
fn custom_driver_is_listed_before_builtins_and_exposes_catalog() {
    let launcher = Launcher::builder()
        .build()
        .register_driver(DriverRegistration::new(
            CUSTOM,
            Arc::new(VanillaDriver::with_defaults()),
            DriverCatalog::game_only(CUSTOM, VanillaCatalog::with_defaults()),
        ))
        .unwrap();

    assert_eq!(launcher.drivers().descriptors().next(), Some(CUSTOM));
    assert_eq!(launcher.drivers().catalog(CUSTOM).unwrap().driver(), CUSTOM);
    assert!(launcher.drivers().catalog(VANILLA_DRIVER).is_ok());
}

#[tokio::test]
async fn custom_driver_runs_during_instance_inspection() {
    let root = common::temp_root("elemental-custom-inspect");
    let directory = root.join("versions").join("custom");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("custom.json"), VALID_METADATA).unwrap();
    std::fs::write(directory.join("custom.jar"), []).unwrap();
    let launcher = Launcher::builder()
        .storage_root(root.clone())
        .build()
        .register_driver(DriverRegistration::new(
            CUSTOM,
            Arc::new(DetectsEverything),
            DriverCatalog::game_only(CUSTOM, VanillaCatalog::with_defaults()),
        ))
        .unwrap();

    let report = launcher.inspect_instances().await.unwrap();

    assert_eq!(report.instances.len(), 1);
    assert_eq!(report.instances[0].instance_name, "custom");
    assert_eq!(report.instances[0].driver.driver, CUSTOM);
    assert!(
        launcher
            .drivers()
            .descriptors()
            .any(|driver| driver == CUSTOM)
    );
    common::remove(&root);
}
