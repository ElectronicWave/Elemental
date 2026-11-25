#![cfg_attr(
    not(feature = "driver-all"),
    allow(unused_imports, dead_code, unreachable_code, unused_variables)
)]
#![allow(unreachable_patterns)]

use std::{fmt, sync::Arc};

use elemental_driver::{
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        LITELOADER_DRIVER, NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER,
        VANILLA_DRIVER,
    },
    driver::{
        Driver, DriverDescriptor, InstallerFamilyDriver, ProfiledVersionJsonDriver,
        ProfiledVersionJsonFamily, ProfiledVersionJsonFamilyExt,
    },
    families::{
        installer::{InstallerMavenSource, MavenInstallerSpec},
        version_json::LoaderMetaSource,
    },
};

use crate::{
    catalog::DriverCatalog,
    error::{Error, Result},
    origin::DriverOrigins,
};

#[cfg(feature = "driver-cleanroom")]
use elemental_driver::drivers::cleanroom::{self, CLEANROOM_SPEC};
#[cfg(any(
    feature = "driver-fabric",
    feature = "driver-legacyfabric",
    feature = "driver-babric"
))]
use elemental_driver::drivers::fabric::{
    driver::FabricDriverFamily,
    source::{FabricEndpoints, FabricFlavor},
};
#[cfg(feature = "driver-forge")]
use elemental_driver::drivers::forge::{self, FORGE_SPEC};
#[cfg(feature = "driver-liteloader")]
use elemental_driver::drivers::liteloader::{
    catalog::LiteLoaderCatalog,
    driver::LiteLoaderDriverFamily,
    source::{LiteLoaderEndpoints, LiteLoaderSource},
};
#[cfg(feature = "driver-neoforge")]
use elemental_driver::drivers::neoforge::{self, NEOFORGE_SPEC};
#[cfg(feature = "driver-outlands")]
use elemental_driver::drivers::outlands::{self, OUTLANDS_SPEC};
#[cfg(feature = "driver-quilt")]
use elemental_driver::drivers::quilt::{
    QuiltCatalog, driver::QuiltDriverFamily, source::QuiltEndpoints,
};
#[cfg(feature = "driver-rift")]
use elemental_driver::drivers::rift::{
    catalog::RiftCatalog,
    driver::RiftDriverFamily,
    source::{RiftEndpoints, RiftSource},
};
use elemental_driver::drivers::vanilla::{catalog::VanillaCatalog, driver::VanillaDriver};

/// A third-party driver added to a [`crate::Launcher`].
///
/// The launcher keeps the driver instance, so the driver must be safe to share
/// and must not depend on mutable per-call state.
#[derive(Clone)]
pub struct DriverRegistration {
    descriptor: DriverDescriptor,
    driver: Arc<dyn Driver>,
    catalog: DriverCatalog,
}

impl DriverRegistration {
    /// Creates a registration for a driver that is not built into Elemental.
    pub fn new(
        descriptor: DriverDescriptor,
        driver: Arc<dyn Driver>,
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

    pub(crate) fn driver(&self) -> Arc<dyn Driver> {
        Arc::clone(&self.driver)
    }
}

impl fmt::Debug for DriverRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DriverRegistration")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

/// Built-in and third-party drivers known to a [`crate::Launcher`].
#[derive(Clone, Debug)]
pub struct DriverRegistry {
    custom: Vec<DriverRegistration>,
}

impl DriverRegistry {
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
    ) -> Result<Arc<dyn Driver>> {
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

    pub(crate) fn register(&mut self, registration: DriverRegistration) -> Result<()> {
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
            Self::Forge => DriverCatalog::loader(descriptor, forge::catalog()),
            #[cfg(feature = "driver-cleanroom")]
            Self::Cleanroom => DriverCatalog::loader(descriptor, cleanroom::catalog()),
            #[cfg(feature = "driver-outlands")]
            Self::Outlands => DriverCatalog::loader(descriptor, outlands::catalog()),
            #[cfg(feature = "driver-neoforge")]
            Self::NeoForge => DriverCatalog::loader(descriptor, neoforge::catalog()),
            _ => return Err(builtin_unsupported(descriptor)),
        })
    }

    pub(crate) fn build(
        &self,
        descriptor: DriverDescriptor,
        origins: &DriverOrigins,
    ) -> Result<Box<dyn Driver>> {
        let policy = origins.policy_for(descriptor);
        match self {
            Self::Vanilla => Ok(Box::new(VanillaDriver::new(origins.vanilla_source()))),
            #[cfg(any(
                feature = "driver-fabric",
                feature = "driver-legacyfabric",
                feature = "driver-babric"
            ))]
            Self::Fabric(flavor) => {
                let endpoints = FabricEndpoints::for_flavor_with_policy(flavor.clone(), &policy)?;
                Ok(Box::new(profiled_driver(
                    origins,
                    FabricDriverFamily::new(flavor.clone()),
                    LoaderMetaSource::new(endpoints),
                )))
            }
            #[cfg(feature = "driver-quilt")]
            Self::Quilt => Ok(Box::new(profiled_driver(
                origins,
                QuiltDriverFamily,
                LoaderMetaSource::new(QuiltEndpoints::new(policy)),
            ))),
            #[cfg(feature = "driver-liteloader")]
            Self::LiteLoader => Ok(Box::new(profiled_driver(
                origins,
                LiteLoaderDriverFamily,
                LiteLoaderSource::new(LiteLoaderEndpoints::new(policy)),
            ))),
            #[cfg(feature = "driver-rift")]
            Self::Rift => Ok(Box::new(profiled_driver(
                origins,
                RiftDriverFamily,
                RiftSource::new(RiftEndpoints::new(policy)),
            ))),
            #[cfg(feature = "driver-forge")]
            Self::Forge => Ok(Box::new(installer_driver(origins, &FORGE_SPEC))),
            #[cfg(feature = "driver-cleanroom")]
            Self::Cleanroom => Ok(Box::new(installer_driver(origins, &CLEANROOM_SPEC))),
            #[cfg(feature = "driver-outlands")]
            Self::Outlands => Ok(Box::new(installer_driver(origins, &OUTLANDS_SPEC))),
            #[cfg(feature = "driver-neoforge")]
            Self::NeoForge => Ok(Box::new(installer_driver(origins, &NEOFORGE_SPEC))),
            _ => Err(builtin_unsupported(descriptor)),
        }
    }
}

fn profiled_driver<F>(
    origins: &DriverOrigins,
    family: F,
    source: F::Source,
) -> ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    family
        .build_driver(source)
        .with_vanilla_source(origins.vanilla_source())
}

fn installer_driver(
    origins: &DriverOrigins,
    spec: &'static MavenInstallerSpec,
) -> InstallerFamilyDriver {
    InstallerFamilyDriver::new(InstallerMavenSource::with_policy(
        spec,
        origins.policy_for(spec.descriptor()),
    ))
    .with_vanilla_source(origins.vanilla_source())
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
