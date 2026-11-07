use elemental_driver::{
    descriptors::{BABRIC_DRIVER, FABRIC_DRIVER, LEGACY_FABRIC_DRIVER},
    drivers::{
        cleanroom::{CleanroomDriverSpec, CleanroomEndpoints, CleanroomOrigin, CleanroomSource},
        fabric::{
            driver::FabricDriverFamily,
            source::{FabricEndpoints, FabricFlavor, FabricOrigin, FabricSource},
        },
        forge::{ForgeEndpoints, ForgeOrigin, ForgeSource},
        liteloader::{
            driver::LiteLoaderDriverFamily,
            source::{LiteLoaderEndpoints, LiteLoaderOrigin, LiteLoaderSource},
        },
        neoforge::{NeoForgeEndpoints, NeoForgeOrigin, NeoForgeSource},
        outlands::{OutlandsDriverSpec, OutlandsEndpoints, OutlandsOrigin, OutlandsSource},
        quilt::{
            driver::QuiltDriverFamily,
            source::{QuiltEndpoints, QuiltOrigin, QuiltSource},
        },
        rift::{
            driver::RiftDriverFamily,
            source::{RiftEndpoints, RiftOrigin, RiftSource},
        },
        vanilla::source::{VanillaEndpoints, VanillaOrigin, VanillaSource},
    },
    families::{installer::InstallerFamily, version_json::ProfiledVersionJsonFamily},
    url::{OriginEndpoints, OriginPolicy},
};

use crate::error::{Error, Result};

/// Origin overrides grouped by driver.
///
/// Each driver family exposes its own typed origin enum, so the configuration
/// stays typed instead of collapsing every origin into one launcher-wide enum.
#[derive(Debug, Clone, Default)]
pub struct DriverOrigins {
    pub(crate) vanilla: OriginPolicy<VanillaOrigin>,
    pub(crate) fabric: OriginPolicy<FabricOrigin>,
    pub(crate) legacy_fabric: OriginPolicy<FabricOrigin>,
    pub(crate) babric: OriginPolicy<FabricOrigin>,
    pub(crate) quilt: OriginPolicy<QuiltOrigin>,
    pub(crate) liteloader: OriginPolicy<LiteLoaderOrigin>,
    pub(crate) rift: OriginPolicy<RiftOrigin>,
    pub(crate) forge: OriginPolicy<ForgeOrigin>,
    pub(crate) neoforge: OriginPolicy<NeoForgeOrigin>,
    pub(crate) cleanroom: OriginPolicy<CleanroomOrigin>,
    pub(crate) outlands: OriginPolicy<OutlandsOrigin>,
}

impl DriverOrigins {
    pub fn vanilla(&mut self) -> &mut OriginPolicy<VanillaOrigin> {
        &mut self.vanilla
    }

    pub fn fabric(&mut self) -> &mut OriginPolicy<FabricOrigin> {
        &mut self.fabric
    }

    pub fn legacy_fabric(&mut self) -> &mut OriginPolicy<FabricOrigin> {
        &mut self.legacy_fabric
    }

    pub fn babric(&mut self) -> &mut OriginPolicy<FabricOrigin> {
        &mut self.babric
    }

    pub fn quilt(&mut self) -> &mut OriginPolicy<QuiltOrigin> {
        &mut self.quilt
    }

    pub fn liteloader(&mut self) -> &mut OriginPolicy<LiteLoaderOrigin> {
        &mut self.liteloader
    }

    pub fn rift(&mut self) -> &mut OriginPolicy<RiftOrigin> {
        &mut self.rift
    }

    pub fn forge(&mut self) -> &mut OriginPolicy<ForgeOrigin> {
        &mut self.forge
    }

    pub fn neoforge(&mut self) -> &mut OriginPolicy<NeoForgeOrigin> {
        &mut self.neoforge
    }

    pub fn cleanroom(&mut self) -> &mut OriginPolicy<CleanroomOrigin> {
        &mut self.cleanroom
    }

    pub fn outlands(&mut self) -> &mut OriginPolicy<OutlandsOrigin> {
        &mut self.outlands
    }
}

impl DriverOrigins {
    pub(crate) fn vanilla_source(&self) -> VanillaSource {
        VanillaSource::new(VanillaEndpoints::new(self.vanilla.clone()))
    }
}

pub(crate) trait ProfiledFamilySource: ProfiledVersionJsonFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source>;
}

impl ProfiledFamilySource for FabricDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        let (flavor, overrides) = match self.descriptor() {
            FABRIC_DRIVER => (FabricFlavor::Fabric, &origins.fabric),
            LEGACY_FABRIC_DRIVER => (FabricFlavor::LegacyFabric, &origins.legacy_fabric),
            BABRIC_DRIVER => (FabricFlavor::Babric, &origins.babric),
            descriptor => {
                return Err(Error::UnsupportedDriver {
                    id: descriptor.id.to_owned(),
                });
            }
        };

        let base = FabricEndpoints::for_flavor(flavor)?;
        Ok(FabricSource::new(FabricEndpoints::new(
            base.origin_policy().merged_with(overrides),
        )))
    }
}

impl ProfiledFamilySource for QuiltDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(QuiltSource::new(QuiltEndpoints::new(origins.quilt.clone())))
    }
}

impl ProfiledFamilySource for LiteLoaderDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(LiteLoaderSource::new(LiteLoaderEndpoints::new(
            origins.liteloader.clone(),
        )))
    }
}

impl ProfiledFamilySource for RiftDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(RiftSource::new(RiftEndpoints::new(origins.rift.clone())))
    }
}

pub(crate) trait InstallerFamilySource: InstallerFamily {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source>;
}

impl InstallerFamilySource for elemental_driver::drivers::forge::ForgeFamily {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(ForgeSource::new(ForgeEndpoints::new(OriginEndpoints::new(
            origins.forge.clone(),
        ))))
    }
}

impl InstallerFamilySource for elemental_driver::drivers::neoforge::NeoForgeFamily {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(NeoForgeSource::new(NeoForgeEndpoints::new(
            OriginEndpoints::new(origins.neoforge.clone()),
        )))
    }
}

impl InstallerFamilySource for CleanroomDriverSpec {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(CleanroomSource::new(CleanroomEndpoints::new(
            OriginEndpoints::new(origins.cleanroom.clone()),
        )))
    }
}

impl InstallerFamilySource for OutlandsDriverSpec {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(OutlandsSource::new(OutlandsEndpoints::new(
            OriginEndpoints::new(origins.outlands.clone()),
        )))
    }
}
