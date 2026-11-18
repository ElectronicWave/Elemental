#![cfg_attr(
    not(feature = "driver-all"),
    allow(unused_imports, dead_code, unreachable_code, unused_variables)
)]
#![allow(unreachable_patterns)]

use std::{fmt, sync::Arc};

use elemental_core::storage::layout::Layout;
use elemental_driver::{
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::{Driver, DriverDescriptor},
    families::{
        installer::InstallerFamilyDriver,
        version_json::{
            BaseInstanceLayout, BaseRootLayout, ProfiledVersionJsonDriver,
            ProfiledVersionJsonFamilyExt, VersionJsonInstanceLayout, VersionJsonRootLayout,
        },
    },
};

use crate::{
    catalog::DriverCatalog,
    error::{Error, Result},
    origin::{DriverOrigins, InstallerFamilySource, ProfiledFamilySource},
};

#[cfg(feature = "driver-cleanroom")]
use elemental_driver::drivers::cleanroom::{CleanroomCatalog, CleanroomDriverSpec};
#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use elemental_driver::drivers::fabric::{driver::FabricDriverFamily, source::FabricFlavor};
#[cfg(feature = "driver-forge")]
use elemental_driver::drivers::forge::{ForgeCatalog, ForgeFamily};
#[cfg(feature = "driver-liteloader")]
use elemental_driver::drivers::liteloader::{
    catalog::LiteLoaderCatalog, driver::LiteLoaderDriverFamily,
};
#[cfg(feature = "driver-neoforge")]
use elemental_driver::drivers::neoforge::{NeoForgeCatalog, NeoForgeFamily};
#[cfg(feature = "driver-outlands")]
use elemental_driver::drivers::outlands::{OutlandsCatalog, OutlandsDriverSpec};
#[cfg(feature = "driver-quilt")]
use elemental_driver::drivers::quilt::{QuiltCatalog, driver::QuiltDriverFamily};
#[cfg(feature = "driver-rift")]
use elemental_driver::drivers::rift::{catalog::RiftCatalog, driver::RiftDriverFamily};
use elemental_driver::drivers::vanilla::{catalog::VanillaCatalog, driver::VanillaDriver};

/// A third-party driver added to a [`crate::Launcher`].
///
/// The launcher keeps the driver instance, so the driver must be safe to share
/// and must not depend on mutable per-call state.
pub struct DriverRegistration<L, VL>
where
    L: Layout,
    VL: Layout,
{
    descriptor: DriverDescriptor,
    driver: Arc<dyn Driver<L, VL>>,
    catalog: DriverCatalog,
}

impl<L, VL> DriverRegistration<L, VL>
where
    L: Layout,
    VL: Layout,
{
    /// Creates a registration for a driver that is not built into Elemental.
    pub fn new(
        descriptor: DriverDescriptor,
        driver: Arc<dyn Driver<L, VL>>,
        catalog: DriverCatalog,
    ) -> Self {
        Self {
            descriptor,
            driver,
            catalog,
        }
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        self.descriptor
    }

    pub(crate) fn driver(&self) -> Arc<dyn Driver<L, VL>> {
        Arc::clone(&self.driver)
    }
}

impl<L, VL> Clone for DriverRegistration<L, VL>
where
    L: Layout,
    VL: Layout,
{
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor,
            driver: Arc::clone(&self.driver),
            catalog: self.catalog.clone(),
        }
    }
}

impl<L, VL> fmt::Debug for DriverRegistration<L, VL>
where
    L: Layout,
    VL: Layout,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DriverRegistration")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

/// Built-in and third-party drivers known to a [`crate::Launcher`].
#[derive(Clone, Debug)]
pub struct DriverRegistry<L = BaseRootLayout, VL = BaseInstanceLayout>
where
    L: Layout,
    VL: Layout,
{
    custom: Vec<DriverRegistration<L, VL>>,
}

impl<L, VL> DriverRegistry<L, VL>
where
    L: Layout,
    VL: Layout,
{
    pub(crate) fn new() -> Self {
        Self { custom: Vec::new() }
    }

    /// Returns third-party drivers first, then built-in drivers in detection order.
    pub fn descriptors(&self) -> impl Iterator<Item = DriverDescriptor> + '_ {
        self.custom
            .iter()
            .map(DriverRegistration::descriptor)
            .chain(
                BUILTIN_DRIVERS
                    .iter()
                    .map(|registration| registration.descriptor),
            )
    }

    /// Returns the version catalog for a built-in or registered driver.
    pub fn catalog(&self, descriptor: DriverDescriptor) -> Result<DriverCatalog> {
        if let Some(registration) = self
            .custom
            .iter()
            .find(|registration| registration.descriptor() == descriptor)
        {
            return Ok(registration.catalog.clone());
        }

        builtin_registration(descriptor)
            .ok_or_else(|| builtin_unsupported(descriptor))?
            .driver
            .catalog(descriptor)
    }

    pub(crate) fn driver(
        &self,
        descriptor: DriverDescriptor,
        origins: &DriverOrigins,
    ) -> Result<Arc<dyn Driver<L, VL>>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone + Send,
    {
        if let Some(registration) = self
            .custom
            .iter()
            .find(|registration| registration.descriptor() == descriptor)
        {
            return Ok(registration.driver());
        }

        let registration =
            builtin_registration(descriptor).ok_or_else(|| builtin_unsupported(descriptor))?;
        Ok(Arc::from(registration.driver.build(descriptor, origins)?))
    }

    pub(crate) fn register(&mut self, registration: DriverRegistration<L, VL>) -> Result<()> {
        let descriptor = registration.descriptor();
        if builtin_registration(descriptor).is_some()
            || self
                .custom
                .iter()
                .any(|registered| registered.descriptor() == descriptor)
        {
            return Err(Error::DriverAlreadyRegistered {
                id: descriptor.id.to_string(),
            });
        }

        self.custom.push(registration);
        Ok(())
    }
}

pub(crate) struct BuiltinDriverRegistration {
    pub(crate) descriptor: DriverDescriptor,
    pub(crate) driver: BuiltinDriver,
}

#[derive(Clone)]
pub(crate) enum BuiltinDriver {
    Vanilla,
    #[cfg(any(
        feature = "driver-fabric",
        feature = "driver-legacyfabric",
        feature = "driver-babric"
    ))]
    Fabric(FabricFlavor),
    #[cfg(feature = "driver-quilt")]
    Quilt,
    #[cfg(feature = "driver-liteloader")]
    LiteLoader,
    #[cfg(feature = "driver-rift")]
    Rift,
    #[cfg(feature = "driver-forge")]
    Forge,
    #[cfg(feature = "driver-cleanroom")]
    Cleanroom,
    #[cfg(feature = "driver-outlands")]
    Outlands,
    #[cfg(feature = "driver-neoforge")]
    NeoForge,
}

impl BuiltinDriver {
    pub(crate) fn catalog(&self, descriptor: DriverDescriptor) -> Result<DriverCatalog> {
        Ok(match self {
            Self::Vanilla => DriverCatalog::game_only(descriptor, VanillaCatalog::with_defaults()),
            #[cfg(any(
                feature = "driver-fabric",
                feature = "driver-legacyfabric",
                feature = "driver-babric"
            ))]
            Self::Fabric(flavor) => DriverCatalog::with_loader(descriptor, flavor.clone())?,
            #[cfg(feature = "driver-quilt")]
            Self::Quilt => DriverCatalog::loader(descriptor, QuiltCatalog::with_defaults()),
            #[cfg(feature = "driver-liteloader")]
            Self::LiteLoader => {
                DriverCatalog::loader(descriptor, LiteLoaderCatalog::with_defaults())
            }
            #[cfg(feature = "driver-rift")]
            Self::Rift => DriverCatalog::loader(descriptor, RiftCatalog::with_defaults()),
            #[cfg(feature = "driver-forge")]
            Self::Forge => DriverCatalog::loader(descriptor, ForgeCatalog::with_defaults()),
            #[cfg(feature = "driver-cleanroom")]
            Self::Cleanroom => DriverCatalog::loader(descriptor, CleanroomCatalog::with_defaults()),
            #[cfg(feature = "driver-outlands")]
            Self::Outlands => DriverCatalog::loader(descriptor, OutlandsCatalog::with_defaults()),
            #[cfg(feature = "driver-neoforge")]
            Self::NeoForge => DriverCatalog::loader(descriptor, NeoForgeCatalog::with_defaults()),
            _ => return Err(builtin_unsupported(descriptor)),
        })
    }

    pub(crate) fn build<L, VL>(
        &self,
        descriptor: DriverDescriptor,
        origins: &DriverOrigins,
    ) -> Result<Box<dyn Driver<L, VL>>>
    where
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone + Send,
    {
        Ok(match self {
            Self::Vanilla => Box::new(VanillaDriver::new(origins.vanilla_source())),
            #[cfg(any(
                feature = "driver-fabric",
                feature = "driver-legacyfabric",
                feature = "driver-babric"
            ))]
            Self::Fabric(flavor) => Box::new(profiled_driver(
                origins,
                FabricDriverFamily::new(flavor.clone()),
            )?),
            #[cfg(feature = "driver-quilt")]
            Self::Quilt => Box::new(profiled_driver(origins, QuiltDriverFamily)?),
            #[cfg(feature = "driver-liteloader")]
            Self::LiteLoader => Box::new(profiled_driver(origins, LiteLoaderDriverFamily)?),
            #[cfg(feature = "driver-rift")]
            Self::Rift => Box::new(profiled_driver(origins, RiftDriverFamily)?),
            #[cfg(feature = "driver-forge")]
            Self::Forge => Box::new(installer_driver::<ForgeFamily>(origins)?),
            #[cfg(feature = "driver-cleanroom")]
            Self::Cleanroom => Box::new(installer_driver::<CleanroomDriverSpec>(origins)?),
            #[cfg(feature = "driver-outlands")]
            Self::Outlands => Box::new(installer_driver::<OutlandsDriverSpec>(origins)?),
            #[cfg(feature = "driver-neoforge")]
            Self::NeoForge => Box::new(installer_driver::<NeoForgeFamily>(origins)?),
            _ => return Err(builtin_unsupported(descriptor)),
        })
    }
}

fn profiled_driver<F>(origins: &DriverOrigins, family: F) -> Result<ProfiledVersionJsonDriver<F>>
where
    F: ProfiledFamilySource,
{
    Ok(family
        .clone()
        .build_driver(family.source_with_origins(origins)?)
        .with_vanilla_source(origins.vanilla_source()))
}

fn installer_driver<F>(origins: &DriverOrigins) -> Result<InstallerFamilyDriver<F>>
where
    F: InstallerFamilySource,
{
    Ok(InstallerFamilyDriver::new(F::source_with_origins(origins)?)
        .with_vanilla_source(origins.vanilla_source()))
}

/// Built-in drivers in instance-detection order.
pub(crate) const BUILTIN_DRIVERS: &[BuiltinDriverRegistration] = &[
    #[cfg(feature = "driver-fabric")]
    BuiltinDriverRegistration {
        descriptor: FABRIC_DRIVER,
        driver: BuiltinDriver::Fabric(FabricFlavor::Fabric),
    },
    #[cfg(feature = "driver-legacyfabric")]
    BuiltinDriverRegistration {
        descriptor: LEGACY_FABRIC_DRIVER,
        driver: BuiltinDriver::Fabric(FabricFlavor::LegacyFabric),
    },
    #[cfg(feature = "driver-babric")]
    BuiltinDriverRegistration {
        descriptor: BABRIC_DRIVER,
        driver: BuiltinDriver::Fabric(FabricFlavor::Babric),
    },
    #[cfg(feature = "driver-quilt")]
    BuiltinDriverRegistration {
        descriptor: QUILT_DRIVER,
        driver: BuiltinDriver::Quilt,
    },
    #[cfg(feature = "driver-liteloader")]
    BuiltinDriverRegistration {
        descriptor: LITELOADER_DRIVER,
        driver: BuiltinDriver::LiteLoader,
    },
    #[cfg(feature = "driver-rift")]
    BuiltinDriverRegistration {
        descriptor: RIFT_DRIVER,
        driver: BuiltinDriver::Rift,
    },
    #[cfg(feature = "driver-forge")]
    BuiltinDriverRegistration {
        descriptor: FORGE_DRIVER,
        driver: BuiltinDriver::Forge,
    },
    #[cfg(feature = "driver-cleanroom")]
    BuiltinDriverRegistration {
        descriptor: CLEANROOM_DRIVER,
        driver: BuiltinDriver::Cleanroom,
    },
    #[cfg(feature = "driver-outlands")]
    BuiltinDriverRegistration {
        descriptor: OUTLANDS_DRIVER,
        driver: BuiltinDriver::Outlands,
    },
    #[cfg(feature = "driver-neoforge")]
    BuiltinDriverRegistration {
        descriptor: NEOFORGE_DRIVER,
        driver: BuiltinDriver::NeoForge,
    },
    BuiltinDriverRegistration {
        descriptor: VANILLA_DRIVER,
        driver: BuiltinDriver::Vanilla,
    },
];

pub(crate) fn builtin_registration(
    descriptor: DriverDescriptor,
) -> Option<&'static BuiltinDriverRegistration> {
    BUILTIN_DRIVERS
        .iter()
        .find(|registration| registration.descriptor == descriptor)
}

pub(crate) fn builtin_unsupported(descriptor: DriverDescriptor) -> Error {
    Error::UnsupportedDriver {
        id: descriptor.id.to_string(),
    }
}

#[cfg(all(test, feature = "driver-all"))]
mod tests {
    use elemental_driver::descriptors::ALL_DRIVERS;

    use super::*;

    #[test]
    fn builtin_registry_matches_driver_detection_order() {
        let registered = BUILTIN_DRIVERS
            .iter()
            .map(|registration| registration.descriptor)
            .collect::<Vec<_>>();

        assert_eq!(registered, ALL_DRIVERS);
    }
}
