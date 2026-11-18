use crate::driver::DriverDescriptor;

pub const VANILLA_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "vanilla",
    name: "Vanilla",
};

pub const FABRIC_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "fabric",
    name: "Fabric",
};

pub const LEGACY_FABRIC_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "legacyfabric",
    name: "LegacyFabric",
};

pub const BABRIC_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "babric",
    name: "Babric",
};

pub const QUILT_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "quilt",
    name: "Quilt",
};

pub const LITELOADER_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "liteloader",
    name: "LiteLoader",
};

pub const RIFT_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "rift",
    name: "Rift",
};

pub const FORGE_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "forge",
    name: "Forge",
};

pub const CLEANROOM_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "cleanroom",
    name: "Cleanroom",
};

pub const OUTLANDS_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "outlands",
    name: "Outlands",
};

pub const NEOFORGE_DRIVER: DriverDescriptor = DriverDescriptor {
    id: "neoforge",
    name: "NeoForge",
};

/// All supported drivers in instance-detection order.
///
/// More specific drivers come before `Vanilla`, so a modded instance is not
/// claimed by the vanilla fallback first.
pub const ALL_DRIVERS: [DriverDescriptor; 11] = [
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
    VANILLA_DRIVER,
];

/// Resolves a known driver descriptor by its stable id.
pub fn from_id(id: &str) -> Option<DriverDescriptor> {
    ALL_DRIVERS
        .into_iter()
        .find(|descriptor| descriptor.id == id)
}
