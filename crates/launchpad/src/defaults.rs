use elemental::driver::{
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::DriverDescriptor,
};

pub const CLEANROOM_LAUNCHPAD_RUNTIME_MAJOR_VERSION: usize = 25;
pub const OUTLANDS_LAUNCHPAD_RUNTIME_MAJOR_VERSION: usize = 25;

pub const SUPPORTED_DRIVERS: [DriverDescriptor; 11] = [
    VANILLA_DRIVER,
    FABRIC_DRIVER,
    LEGACY_FABRIC_DRIVER,
    BABRIC_DRIVER,
    QUILT_DRIVER,
    LITELOADER_DRIVER,
    RIFT_DRIVER,
    FORGE_DRIVER,
    CLEANROOM_DRIVER,
    OUTLANDS_DRIVER,
    NEOFORGE_DRIVER,
];

#[derive(Clone, Copy, Debug)]
pub struct DriverDefaults {
    pub game_version: &'static str,
    pub loader_version: Option<&'static str>,
    pub instance_prefix: &'static str,
    pub runtime_major_version: Option<usize>,
}

pub fn driver_defaults(driver: DriverDescriptor) -> Option<DriverDefaults> {
    match driver {
        VANILLA_DRIVER => Some(DriverDefaults {
            game_version: "1.20.1",
            loader_version: None,
            instance_prefix: "MyVanilla",
            runtime_major_version: None,
        }),
        FABRIC_DRIVER => Some(DriverDefaults {
            game_version: "1.20.1",
            loader_version: Some("0.16.10"),
            instance_prefix: "MyFabric",
            runtime_major_version: None,
        }),
        LEGACY_FABRIC_DRIVER => Some(DriverDefaults {
            game_version: "1.20.1",
            loader_version: Some("0.16.10"),
            instance_prefix: "MyLegacyFabric",
            runtime_major_version: None,
        }),
        BABRIC_DRIVER => Some(DriverDefaults {
            game_version: "1.20.1",
            loader_version: Some("0.16.10"),
            instance_prefix: "MyBabric",
            runtime_major_version: None,
        }),
        QUILT_DRIVER => Some(DriverDefaults {
            game_version: "1.20.1",
            loader_version: Some("0.24.0"),
            instance_prefix: "MyQuilt",
            runtime_major_version: None,
        }),
        LITELOADER_DRIVER => Some(DriverDefaults {
            game_version: "1.7.10",
            loader_version: Some("1.7.10_04"),
            instance_prefix: "MyLiteLoader",
            runtime_major_version: None,
        }),
        RIFT_DRIVER => Some(DriverDefaults {
            game_version: "1.13.2",
            loader_version: Some("1.0.4-106"),
            instance_prefix: "MyRift",
            runtime_major_version: None,
        }),
        FORGE_DRIVER => Some(DriverDefaults {
            game_version: "1.20.1",
            loader_version: Some("47.3.1"),
            instance_prefix: "MyForge",
            runtime_major_version: None,
        }),
        CLEANROOM_DRIVER => Some(DriverDefaults {
            game_version: "1.12.2",
            loader_version: Some("0.5.8-alpha"),
            instance_prefix: "MyCleanroom",
            runtime_major_version: Some(CLEANROOM_LAUNCHPAD_RUNTIME_MAJOR_VERSION),
        }),
        OUTLANDS_DRIVER => Some(DriverDefaults {
            game_version: "1.12.2",
            loader_version: Some("2026.11.07"),
            instance_prefix: "MyOutlands",
            runtime_major_version: Some(OUTLANDS_LAUNCHPAD_RUNTIME_MAJOR_VERSION),
        }),
        NEOFORGE_DRIVER => Some(DriverDefaults {
            game_version: "1.21.1",
            loader_version: Some("21.1.199"),
            instance_prefix: "MyNeoForge",
            runtime_major_version: None,
        }),
        _ => None,
    }
}

pub fn default_instance_name(defaults: DriverDefaults, game_version: &str) -> String {
    format!("{}-{game_version}", defaults.instance_prefix)
}
