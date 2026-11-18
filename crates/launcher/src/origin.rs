#![cfg_attr(not(feature = "driver-all"), allow(dead_code))]

use elemental_driver::{
    drivers::vanilla::source::{VanillaEndpoints, VanillaOrigin, VanillaSource},
    families::{installer::InstallerFamily, version_json::ProfiledVersionJsonFamily},
    url::OriginPolicy,
};

#[cfg(feature = "driver-babric")]
use elemental_driver::descriptors::BABRIC_DRIVER;
#[cfg(feature = "driver-fabric")]
use elemental_driver::descriptors::FABRIC_DRIVER;
#[cfg(feature = "driver-legacyfabric")]
use elemental_driver::descriptors::LEGACY_FABRIC_DRIVER;
#[cfg(feature = "driver-cleanroom")]
use elemental_driver::drivers::cleanroom::{
    CleanroomDriverSpec, CleanroomEndpoints, CleanroomOrigin, CleanroomSource,
};
#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use elemental_driver::drivers::fabric::{
    driver::FabricDriverFamily,
    source::{FabricEndpoints, FabricFlavor, FabricOrigin, FabricSource},
};
#[cfg(feature = "driver-forge")]
use elemental_driver::drivers::forge::{ForgeEndpoints, ForgeOrigin, ForgeSource};
#[cfg(feature = "driver-liteloader")]
use elemental_driver::drivers::liteloader::{
    driver::LiteLoaderDriverFamily,
    source::{LiteLoaderEndpoints, LiteLoaderOrigin, LiteLoaderSource},
};
#[cfg(feature = "driver-neoforge")]
use elemental_driver::drivers::neoforge::{NeoForgeEndpoints, NeoForgeOrigin, NeoForgeSource};
#[cfg(feature = "driver-outlands")]
use elemental_driver::drivers::outlands::{
    OutlandsDriverSpec, OutlandsEndpoints, OutlandsOrigin, OutlandsSource,
};
#[cfg(feature = "driver-quilt")]
use elemental_driver::drivers::quilt::{
    driver::QuiltDriverFamily,
    source::{QuiltEndpoints, QuiltOrigin, QuiltSource},
};
#[cfg(feature = "driver-rift")]
use elemental_driver::drivers::rift::{
    driver::RiftDriverFamily,
    source::{RiftEndpoints, RiftOrigin, RiftSource},
};
#[cfg(any(
    feature = "driver-forge",
    feature = "driver-neoforge",
    feature = "driver-cleanroom",
    feature = "driver-outlands"
))]
use elemental_driver::url::OriginEndpoints;

#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use crate::error::Error;
use crate::error::Result;

/// Origin overrides grouped by driver.
///
/// Each driver family exposes its own typed origin enum, so the configuration
/// stays typed instead of collapsing every origin into one launcher-wide enum.
#[derive(Debug, Clone, Default)]
pub struct DriverOrigins {
    pub(crate) vanilla: OriginPolicy<VanillaOrigin>,
    #[cfg(feature = "driver-fabric")]
    pub(crate) fabric: OriginPolicy<FabricOrigin>,
    #[cfg(feature = "driver-legacyfabric")]
    pub(crate) legacy_fabric: OriginPolicy<FabricOrigin>,
    #[cfg(feature = "driver-babric")]
    pub(crate) babric: OriginPolicy<FabricOrigin>,
    #[cfg(feature = "driver-quilt")]
    pub(crate) quilt: OriginPolicy<QuiltOrigin>,
    #[cfg(feature = "driver-liteloader")]
    pub(crate) liteloader: OriginPolicy<LiteLoaderOrigin>,
    #[cfg(feature = "driver-rift")]
    pub(crate) rift: OriginPolicy<RiftOrigin>,
    #[cfg(feature = "driver-forge")]
    pub(crate) forge: OriginPolicy<ForgeOrigin>,
    #[cfg(feature = "driver-neoforge")]
    pub(crate) neoforge: OriginPolicy<NeoForgeOrigin>,
    #[cfg(feature = "driver-cleanroom")]
    pub(crate) cleanroom: OriginPolicy<CleanroomOrigin>,
    #[cfg(feature = "driver-outlands")]
    pub(crate) outlands: OriginPolicy<OutlandsOrigin>,
}

impl DriverOrigins {
    pub fn vanilla(&mut self) -> &mut OriginPolicy<VanillaOrigin> {
        &mut self.vanilla
    }

    #[cfg(feature = "driver-fabric")]
    pub fn fabric(&mut self) -> &mut OriginPolicy<FabricOrigin> {
        &mut self.fabric
    }

    #[cfg(feature = "driver-legacyfabric")]
    pub fn legacy_fabric(&mut self) -> &mut OriginPolicy<FabricOrigin> {
        &mut self.legacy_fabric
    }

    #[cfg(feature = "driver-babric")]
    pub fn babric(&mut self) -> &mut OriginPolicy<FabricOrigin> {
        &mut self.babric
    }

    #[cfg(feature = "driver-quilt")]
    pub fn quilt(&mut self) -> &mut OriginPolicy<QuiltOrigin> {
        &mut self.quilt
    }

    #[cfg(feature = "driver-liteloader")]
    pub fn liteloader(&mut self) -> &mut OriginPolicy<LiteLoaderOrigin> {
        &mut self.liteloader
    }

    #[cfg(feature = "driver-rift")]
    pub fn rift(&mut self) -> &mut OriginPolicy<RiftOrigin> {
        &mut self.rift
    }

    #[cfg(feature = "driver-forge")]
    pub fn forge(&mut self) -> &mut OriginPolicy<ForgeOrigin> {
        &mut self.forge
    }

    #[cfg(feature = "driver-neoforge")]
    pub fn neoforge(&mut self) -> &mut OriginPolicy<NeoForgeOrigin> {
        &mut self.neoforge
    }

    #[cfg(feature = "driver-cleanroom")]
    pub fn cleanroom(&mut self) -> &mut OriginPolicy<CleanroomOrigin> {
        &mut self.cleanroom
    }

    #[cfg(feature = "driver-outlands")]
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

#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
impl ProfiledFamilySource for FabricDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        let (flavor, overrides) = match self.descriptor() {
            #[cfg(feature = "driver-fabric")]
            FABRIC_DRIVER => (FabricFlavor::Fabric, &origins.fabric),
            #[cfg(feature = "driver-legacyfabric")]
            LEGACY_FABRIC_DRIVER => (FabricFlavor::LegacyFabric, &origins.legacy_fabric),
            #[cfg(feature = "driver-babric")]
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

#[cfg(feature = "driver-quilt")]
impl ProfiledFamilySource for QuiltDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(QuiltSource::new(QuiltEndpoints::new(origins.quilt.clone())))
    }
}

#[cfg(feature = "driver-liteloader")]
impl ProfiledFamilySource for LiteLoaderDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(LiteLoaderSource::new(LiteLoaderEndpoints::new(
            origins.liteloader.clone(),
        )))
    }
}

#[cfg(feature = "driver-rift")]
impl ProfiledFamilySource for RiftDriverFamily {
    fn source_with_origins(&self, origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(RiftSource::new(RiftEndpoints::new(origins.rift.clone())))
    }
}

pub(crate) trait InstallerFamilySource: InstallerFamily {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source>;
}

#[cfg(feature = "driver-forge")]
impl InstallerFamilySource for elemental_driver::drivers::forge::ForgeFamily {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(ForgeSource::new(ForgeEndpoints::new(OriginEndpoints::new(
            origins.forge.clone(),
        ))))
    }
}

#[cfg(feature = "driver-neoforge")]
impl InstallerFamilySource for elemental_driver::drivers::neoforge::NeoForgeFamily {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(NeoForgeSource::new(NeoForgeEndpoints::new(
            OriginEndpoints::new(origins.neoforge.clone()),
        )))
    }
}

#[cfg(feature = "driver-cleanroom")]
impl InstallerFamilySource for CleanroomDriverSpec {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(CleanroomSource::new(CleanroomEndpoints::new(
            OriginEndpoints::new(origins.cleanroom.clone()),
        )))
    }
}

#[cfg(feature = "driver-outlands")]
impl InstallerFamilySource for OutlandsDriverSpec {
    fn source_with_origins(origins: &DriverOrigins) -> Result<Self::Source> {
        Ok(OutlandsSource::new(OutlandsEndpoints::new(
            OriginEndpoints::new(origins.outlands.clone()),
        )))
    }
}
