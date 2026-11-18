#[cfg(feature = "driver-all")]
use std::collections::HashSet;

#[cfg(feature = "driver-all")]
use elemental_driver::descriptors::{ALL_DRIVERS, from_id};
use elemental_launcher::DriverCatalog;

#[cfg(feature = "driver-all")]
#[test]
fn supported_drivers_follow_registry_order() {
    let ids = DriverCatalog::supported_drivers()
        .map(|driver| driver.id)
        .collect::<Vec<_>>();
    let expected = ALL_DRIVERS
        .iter()
        .map(|driver| driver.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, expected);
}

#[cfg(feature = "driver-all")]
#[test]
fn supported_drivers_cover_all_built_in_descriptors() {
    let supported: HashSet<&str> = DriverCatalog::supported_drivers()
        .map(|driver| driver.id)
        .collect();
    let built_in: HashSet<&str> = ALL_DRIVERS.iter().map(|driver| driver.id).collect();

    assert_eq!(supported, built_in);

    for driver in ALL_DRIVERS {
        assert_eq!(from_id(driver.id), Some(*driver));
    }
}
