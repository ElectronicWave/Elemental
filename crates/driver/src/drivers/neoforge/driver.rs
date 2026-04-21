use crate::{
    driver::DriverDescriptor,
    drivers::neoforge::prepared::NeoForgeFamily,
    families::installer::{
        InstallerFamilyDriver, InstallerFamilyDriverSpec, LaunchedInstallerFamilyVersion,
    },
};

pub type NeoForgeDriver = InstallerFamilyDriver<NeoForgeFamily>;
pub type LaunchedNeoForgeVersion<L, VL> = LaunchedInstallerFamilyVersion<NeoForgeFamily, L, VL>;

impl InstallerFamilyDriverSpec for NeoForgeFamily {
    const DRIVER: DriverDescriptor = DriverDescriptor {
        id: "neoforge",
        name: "NeoForge",
    };

    const INSPECT_PREFIXES: &'static [&'static str] = &[
        "net.neoforged:neoforge:",
        "net.neoforged:forge:",
        "net.neoforged:fmlloader:",
    ];
}
