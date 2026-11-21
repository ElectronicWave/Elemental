#![cfg_attr(not(feature = "driver-all"), allow(dead_code))]

use std::collections::HashMap;

#[cfg(feature = "driver-babric")]
use elemental_driver::descriptors::BABRIC_DRIVER;
#[cfg(feature = "driver-cleanroom")]
use elemental_driver::descriptors::CLEANROOM_DRIVER;
#[cfg(feature = "driver-fabric")]
use elemental_driver::descriptors::FABRIC_DRIVER;
#[cfg(feature = "driver-forge")]
use elemental_driver::descriptors::FORGE_DRIVER;
#[cfg(feature = "driver-legacyfabric")]
use elemental_driver::descriptors::LEGACY_FABRIC_DRIVER;
#[cfg(feature = "driver-liteloader")]
use elemental_driver::descriptors::LITELOADER_DRIVER;
#[cfg(feature = "driver-neoforge")]
use elemental_driver::descriptors::NEOFORGE_DRIVER;
#[cfg(feature = "driver-outlands")]
use elemental_driver::descriptors::OUTLANDS_DRIVER;
#[cfg(feature = "driver-quilt")]
use elemental_driver::descriptors::QUILT_DRIVER;
#[cfg(feature = "driver-rift")]
use elemental_driver::descriptors::RIFT_DRIVER;
use elemental_driver::{
    descriptors::VANILLA_DRIVER,
    driver::DriverDescriptor,
    drivers::vanilla::source::{VanillaEndpoints, VanillaSource},
    url::OriginPolicy,
};

/// Mirror and upstream origin overrides, supporting both launcher-wide rules
/// (`global`) and per-driver overrides (`for_driver`).
///
/// Built-in and third-party drivers share the same override mechanism via
/// [`DriverDescriptor`], while convenience accessors preserve discovery and
/// IDE autocompletion for built-in drivers.
#[derive(Debug, Clone, Default)]
pub struct DriverOrigins {
    global: OriginPolicy,
    per_driver: HashMap<&'static str, OriginPolicy>,
}

impl DriverOrigins {
    /// Returns the launcher-wide policy applied before any per-driver overrides.
    pub fn global(&mut self) -> &mut OriginPolicy {
        &mut self.global
    }

    /// Returns a mutable per-driver policy for any built-in or third-party
    /// driver identified by `descriptor`.
    pub fn for_driver(&mut self, descriptor: DriverDescriptor) -> &mut OriginPolicy {
        self.per_driver.entry(descriptor.id).or_default()
    }

    /// Resolves the effective policy for `descriptor` by layering per-driver
    /// overrides on top of [`Self::global`].
    pub fn policy_for(&self, descriptor: DriverDescriptor) -> OriginPolicy {
        match self.per_driver.get(descriptor.id) {
            Some(overrides) => self.global.merged_with(overrides),
            None => self.global.clone(),
        }
    }

    pub fn vanilla(&mut self) -> &mut OriginPolicy {
        self.for_driver(VANILLA_DRIVER)
    }

    #[cfg(feature = "driver-fabric")]
    pub fn fabric(&mut self) -> &mut OriginPolicy {
        self.for_driver(FABRIC_DRIVER)
    }

    #[cfg(feature = "driver-legacyfabric")]
    pub fn legacy_fabric(&mut self) -> &mut OriginPolicy {
        self.for_driver(LEGACY_FABRIC_DRIVER)
    }

    #[cfg(feature = "driver-babric")]
    pub fn babric(&mut self) -> &mut OriginPolicy {
        self.for_driver(BABRIC_DRIVER)
    }

    #[cfg(feature = "driver-quilt")]
    pub fn quilt(&mut self) -> &mut OriginPolicy {
        self.for_driver(QUILT_DRIVER)
    }

    #[cfg(feature = "driver-liteloader")]
    pub fn liteloader(&mut self) -> &mut OriginPolicy {
        self.for_driver(LITELOADER_DRIVER)
    }

    #[cfg(feature = "driver-rift")]
    pub fn rift(&mut self) -> &mut OriginPolicy {
        self.for_driver(RIFT_DRIVER)
    }

    #[cfg(feature = "driver-forge")]
    pub fn forge(&mut self) -> &mut OriginPolicy {
        self.for_driver(FORGE_DRIVER)
    }

    #[cfg(feature = "driver-neoforge")]
    pub fn neoforge(&mut self) -> &mut OriginPolicy {
        self.for_driver(NEOFORGE_DRIVER)
    }

    #[cfg(feature = "driver-cleanroom")]
    pub fn cleanroom(&mut self) -> &mut OriginPolicy {
        self.for_driver(CLEANROOM_DRIVER)
    }

    #[cfg(feature = "driver-outlands")]
    pub fn outlands(&mut self) -> &mut OriginPolicy {
        self.for_driver(OUTLANDS_DRIVER)
    }

    pub(crate) fn vanilla_source(&self) -> VanillaSource {
        VanillaSource::new(VanillaEndpoints::new(self.policy_for(VANILLA_DRIVER)))
    }
}

#[cfg(test)]
mod tests {
    use elemental_driver::{
        descriptors::VANILLA_DRIVER, driver::DriverDescriptor,
        drivers::vanilla::source::VanillaOrigin,
    };

    use super::DriverOrigins;

    const THIRD_PARTY_DRIVER: DriverDescriptor = DriverDescriptor {
        id: "third-party-loader",
        name: "Third Party Loader",
    };

    #[test]
    fn policy_for_layers_per_driver_overrides_over_global_and_supports_custom_drivers() {
        let mut origins = DriverOrigins::default();
        origins
            .global()
            .override_origin(
                VanillaOrigin::LauncherMeta,
                "https://bmclapi2.bangbang93.com".to_owned(),
            )
            .expect("valid global launchermeta mirror");
        origins
            .global()
            .override_origin(
                "https://maven.example.org/releases/",
                "https://mirror.example.org/global-maven".to_owned(),
            )
            .expect("valid global custom mirror");
        origins
            .for_driver(THIRD_PARTY_DRIVER)
            .override_origin(
                "https://maven.example.org/releases",
                "https://mirror.example.org/custom-maven".to_owned(),
            )
            .expect("valid third-party mirror");

        let vanilla_resolved = origins
            .policy_for(VANILLA_DRIVER)
            .resolve(
                VanillaOrigin::LauncherMeta,
                "mc/game/version_manifest_v2.json",
            )
            .expect("resolve vanilla mirror");
        assert_eq!(
            vanilla_resolved,
            "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json"
        );

        let custom_policy = origins.policy_for(THIRD_PARTY_DRIVER);
        let custom_vanilla = custom_policy
            .resolve(
                VanillaOrigin::LauncherMeta,
                "mc/game/version_manifest_v2.json",
            )
            .expect("inherit global override");
        let custom_maven = custom_policy
            .resolve(
                "https://maven.example.org/releases",
                "org/example/loader/1.0.0/loader-1.0.0.pom",
            )
            .expect("per-driver override beats global");
        assert_eq!(
            custom_vanilla,
            "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json"
        );
        assert_eq!(
            custom_maven,
            "https://mirror.example.org/custom-maven/org/example/loader/1.0.0/loader-1.0.0.pom"
        );
    }
}
