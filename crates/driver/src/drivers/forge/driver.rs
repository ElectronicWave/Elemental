use crate::{
    descriptors::FORGE_DRIVER,
    drivers::forge::prepared::ForgeFamily,
    families::installer::{
        InstallerFamilyDriver, InstallerFamilyDriverSpec, LaunchedInstallerFamilyVersion,
    },
};

pub type ForgeDriver = InstallerFamilyDriver<ForgeFamily>;
pub type LaunchedForgeVersion<L, VL> = LaunchedInstallerFamilyVersion<ForgeFamily, L, VL>;

impl InstallerFamilyDriverSpec for ForgeFamily {
    const DRIVER: crate::driver::DriverDescriptor = FORGE_DRIVER;

    const INSPECT_PREFIXES: &'static [&'static str] = &["net.minecraftforge:fmlloader:"];
}
