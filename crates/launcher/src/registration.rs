#![cfg_attr(
    not(feature = "driver-all"),
    allow(unused_imports, dead_code, unreachable_code, unused_variables)
)]
#![allow(unreachable_patterns)]

use std::{fmt, sync::Arc};

use elemental_core::http::SwappableClient;
use elemental_driver::{
    descriptors::{
        BABRIC_DRIVER, CLEANROOM_DRIVER, FABRIC_DRIVER, FORGE_DRIVER, LEGACY_FABRIC_DRIVER,
        NEOFORGE_DRIVER, OUTLANDS_DRIVER, QUILT_DRIVER, RIFT_DRIVER, VANILLA_DRIVER,
    },
    driver::{
        Driver, DriverDescriptor, InstallerFamilyDriver, ProfiledVersionJsonDriver,
        ProfiledVersionJsonFamily, ProfiledVersionJsonFamilyExt,
    },
    drivers::vanilla::source::{VanillaEndpoints, VanillaSource},
    families::{
        installer::{
            InstallerMavenCatalog, InstallerMavenSource, MavenEndpoints, MavenInstallerSpec,
        },
        version_json::LoaderMetaSource,
    },
};

use crate::{
    catalog::DriverCatalog,
    error::{Error, Result},
    origin::DriverOrigins,
};

#[cfg(feature = "driver-cleanroom")]
use elemental_driver::drivers::cleanroom::CLEANROOM_SPEC;
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
use elemental_driver::drivers::forge::FORGE_SPEC;
#[cfg(feature = "driver-neoforge")]
use elemental_driver::drivers::neoforge::NEOFORGE_SPEC;
#[cfg(feature = "driver-outlands")]
use elemental_driver::drivers::outlands::OUTLANDS_SPEC;
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
    http_client: SwappableClient,
}

impl DriverRegistry {
    pub(crate) fn new(http_client: SwappableClient) -> Self {
        Self {
            custom: Vec::new(),
            http_client,
        }
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
            .catalog(descriptor, &self.http_client)
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
        Ok(Arc::from(registration.driver.build(
            descriptor,
            origins,
            &self.http_client,
        )?))
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
    pub(crate) fn catalog(
        &self,
        descriptor: DriverDescriptor,
        http_client: &SwappableClient,
    ) -> Result<DriverCatalog> {
        Ok(match self {
            Self::Vanilla => DriverCatalog::game_only(
                descriptor,
                VanillaCatalog::new(
                    VanillaSource::new(VanillaEndpoints::default())
                        .with_http_client(http_client.clone()),
                ),
            ),
            #[cfg(any(
                feature = "driver-fabric",
                feature = "driver-legacyfabric",
                feature = "driver-babric"
            ))]
            Self::Fabric(flavor) => {
                DriverCatalog::with_loader(descriptor, flavor.clone(), http_client)?
            }
            #[cfg(feature = "driver-quilt")]
            Self::Quilt => DriverCatalog::loader(
                descriptor,
                QuiltCatalog::new(
                    LoaderMetaSource::new(QuiltEndpoints::default())
                        .with_http_client(http_client.clone()),
                ),
            ),
            #[cfg(feature = "driver-rift")]
            Self::Rift => DriverCatalog::loader(
                descriptor,
                RiftCatalog::new(
                    RiftSource::new(RiftEndpoints::default()).with_http_client(http_client.clone()),
                ),
            ),
            #[cfg(feature = "driver-forge")]
            Self::Forge => {
                DriverCatalog::loader(descriptor, installer_catalog(&FORGE_SPEC, http_client))
            }
            #[cfg(feature = "driver-cleanroom")]
            Self::Cleanroom => {
                DriverCatalog::loader(descriptor, installer_catalog(&CLEANROOM_SPEC, http_client))
            }
            #[cfg(feature = "driver-outlands")]
            Self::Outlands => {
                DriverCatalog::loader(descriptor, installer_catalog(&OUTLANDS_SPEC, http_client))
            }
            #[cfg(feature = "driver-neoforge")]
            Self::NeoForge => {
                DriverCatalog::loader(descriptor, installer_catalog(&NEOFORGE_SPEC, http_client))
            }
            _ => return Err(builtin_unsupported(descriptor)),
        })
    }

    pub(crate) fn build(
        &self,
        descriptor: DriverDescriptor,
        origins: &DriverOrigins,
        http_client: &SwappableClient,
    ) -> Result<Box<dyn Driver>> {
        let policy = origins.policy_for(descriptor);
        match self {
            Self::Vanilla => Ok(Box::new(VanillaDriver::new(
                origins.vanilla_source(http_client),
            ))),
            #[cfg(any(
                feature = "driver-fabric",
                feature = "driver-legacyfabric",
                feature = "driver-babric"
            ))]
            Self::Fabric(flavor) => {
                let endpoints = FabricEndpoints::for_flavor_with_policy(flavor.clone(), &policy)?;
                Ok(Box::new(profiled_driver(
                    origins,
                    http_client,
                    FabricDriverFamily::new(flavor.clone()),
                    LoaderMetaSource::new(endpoints).with_http_client(http_client.clone()),
                )))
            }
            #[cfg(feature = "driver-quilt")]
            Self::Quilt => Ok(Box::new(profiled_driver(
                origins,
                http_client,
                QuiltDriverFamily,
                LoaderMetaSource::new(QuiltEndpoints::new(policy))
                    .with_http_client(http_client.clone()),
            ))),
            #[cfg(feature = "driver-rift")]
            Self::Rift => Ok(Box::new(profiled_driver(
                origins,
                http_client,
                RiftDriverFamily,
                RiftSource::new(RiftEndpoints::new(policy)).with_http_client(http_client.clone()),
            ))),
            #[cfg(feature = "driver-forge")]
            Self::Forge => Ok(Box::new(installer_driver(
                origins,
                http_client,
                &FORGE_SPEC,
            ))),
            #[cfg(feature = "driver-cleanroom")]
            Self::Cleanroom => Ok(Box::new(installer_driver(
                origins,
                http_client,
                &CLEANROOM_SPEC,
            ))),
            #[cfg(feature = "driver-outlands")]
            Self::Outlands => Ok(Box::new(installer_driver(
                origins,
                http_client,
                &OUTLANDS_SPEC,
            ))),
            #[cfg(feature = "driver-neoforge")]
            Self::NeoForge => Ok(Box::new(installer_driver(
                origins,
                http_client,
                &NEOFORGE_SPEC,
            ))),
            _ => Err(builtin_unsupported(descriptor)),
        }
    }
}

fn installer_catalog(
    spec: &'static MavenInstallerSpec,
    http_client: &SwappableClient,
) -> InstallerMavenCatalog {
    InstallerMavenCatalog::new(
        InstallerMavenSource::new(MavenEndpoints::official(spec))
            .with_http_client(http_client.clone()),
    )
}

fn profiled_driver<F>(
    origins: &DriverOrigins,
    http_client: &SwappableClient,
    family: F,
    source: F::Source,
) -> ProfiledVersionJsonDriver<F>
where
    F: ProfiledVersionJsonFamily,
{
    family
        .build_driver(source)
        .with_vanilla_source(origins.vanilla_source(http_client))
}

fn installer_driver(
    origins: &DriverOrigins,
    http_client: &SwappableClient,
    spec: &'static MavenInstallerSpec,
) -> InstallerFamilyDriver {
    InstallerFamilyDriver::new(
        InstallerMavenSource::new(MavenEndpoints::new(
            spec,
            origins.policy_for(spec.descriptor()),
        ))
        .with_http_client(http_client.clone()),
    )
    .with_vanilla_source(origins.vanilla_source(http_client))
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
