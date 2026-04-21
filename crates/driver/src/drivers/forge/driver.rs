use crate::{
    driver::DriverDescriptor,
    drivers::forge::prepared::ForgeFamily,
    families::installer::{
        InstallerFamilyDriver, InstallerFamilyDriverSpec, LaunchedInstallerFamilyVersion,
    },
};

pub type ForgeDriver = InstallerFamilyDriver<ForgeFamily>;
pub type LaunchedForgeVersion<L, VL> = LaunchedInstallerFamilyVersion<ForgeFamily, L, VL>;

impl InstallerFamilyDriverSpec for ForgeFamily {
    const DRIVER: DriverDescriptor = DriverDescriptor {
        id: "forge",
        name: "Forge",
    };

    const INSPECT_PREFIXES: &'static [&'static str] = &["net.minecraftforge:fmlloader:"];
}
