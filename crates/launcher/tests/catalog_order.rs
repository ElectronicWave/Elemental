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
