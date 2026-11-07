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

/// Resolves a known driver descriptor by its stable id.
pub fn from_id(id: &str) -> Option<DriverDescriptor> {
    Some(match id {
        "vanilla" => VANILLA_DRIVER,
        "fabric" => FABRIC_DRIVER,
        "legacyfabric" => LEGACY_FABRIC_DRIVER,
        "babric" => BABRIC_DRIVER,
        "quilt" => QUILT_DRIVER,
        "liteloader" => LITELOADER_DRIVER,
        "rift" => RIFT_DRIVER,
        "forge" => FORGE_DRIVER,
        "cleanroom" => CLEANROOM_DRIVER,
        "outlands" => OUTLANDS_DRIVER,
        "neoforge" => NEOFORGE_DRIVER,
        _ => return None,
    })
}
