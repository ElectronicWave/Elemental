use std::collections::HashSet;

use elemental_driver::descriptors::{ALL_DRIVERS, from_id};
use elemental_launcher::DriverCatalog;

#[test]
fn supported_drivers_follow_documented_order() {
    let ids: Vec<&str> = DriverCatalog::supported_drivers()
        .map(|driver| driver.id)
        .collect();

    assert_eq!(
        ids,
        [
            "vanilla",
            "fabric",
            "neoforge",
            "forge",
            "quilt",
            "legacyfabric",
            "babric",
            "outlands",
            "cleanroom",
            "liteloader",
            "rift",
        ]
    );
}

#[test]
fn supported_drivers_cover_all_built_in_descriptors() {
    let supported: HashSet<&str> = DriverCatalog::supported_drivers()
        .map(|driver| driver.id)
        .collect();
    let built_in: HashSet<&str> = ALL_DRIVERS.iter().map(|driver| driver.id).collect();

    assert_eq!(supported, built_in);

    for driver in ALL_DRIVERS {
        assert_eq!(from_id(driver.id), Some(driver));
    }
}
