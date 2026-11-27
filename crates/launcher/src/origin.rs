#![cfg_attr(not(feature = "driver-all"), allow(dead_code))]

#[cfg(feature = "addon-liteloader")]
use elemental_addon::LITELOADER_ADDON_ID;
use elemental_core::url::{OriginPolicy, OriginPolicyMap};
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
};

/// Mirror and upstream origin overrides, supporting both launcher-wide rules
/// (`global`) and per-driver / per-addon overrides.
#[derive(Debug, Clone, Default)]
pub struct DriverOrigins {
    policies: OriginPolicyMap,
}

impl DriverOrigins {
    /// Returns the launcher-wide policy applied before any per-driver or per-addon overrides.
    pub fn global(&mut self) -> &mut OriginPolicy {
        self.policies.global()
    }

    /// Returns a mutable per-driver policy for any built-in or third-party
    /// driver identified by `descriptor`.
    pub fn for_driver(&mut self, descriptor: DriverDescriptor) -> &mut OriginPolicy {
        self.policies.scope(format!("driver:{}", descriptor.id))
    }

    /// Returns a mutable per-addon policy for any built-in or third-party addon ID.
    pub fn for_addon(&mut self, addon_id: impl Into<String>) -> &mut OriginPolicy {
        self.policies.scope(format!("addon:{}", addon_id.into()))
    }

    /// Resolves the effective policy for `descriptor` by layering per-driver
    /// overrides on top of [`Self::global`].
    pub fn policy_for(&self, descriptor: DriverDescriptor) -> OriginPolicy {
        self.policies
            .policy_for(&format!("driver:{}", descriptor.id))
    }

    /// Resolves the effective policy for `addon_id` by layering per-addon
    /// overrides on top of [`Self::global`].
    pub fn policy_for_addon(&self, addon_id: &str) -> OriginPolicy {
        self.policies.policy_for(&format!("addon:{addon_id}"))
    }

    #[cfg(feature = "addon-liteloader")]
    pub fn liteloader_addon(&mut self) -> &mut OriginPolicy {
        self.for_addon(LITELOADER_ADDON_ID)
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

    pub(crate) fn vanilla_source(
        &self,
        http_client: &elemental_core::http::SwappableClient,
    ) -> VanillaSource {
        VanillaSource::with_http_client(
            VanillaEndpoints::new(self.policy_for(VANILLA_DRIVER)),
            http_client.clone(),
        )
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

    #[test]
    fn policy_for_addon_layers_per_addon_overrides_over_global() {
        let mut origins = DriverOrigins::default();
        origins
            .global()
            .override_origin(
                "https://dl.liteloader.com/versions",
                "https://mirror.example.org/global-ll".to_owned(),
            )
            .expect("valid global liteloader mirror");
        origins
            .for_addon("custom-addon")
            .override_origin(
                "https://dl.liteloader.com/versions",
                "https://mirror.example.org/custom-ll".to_owned(),
            )
            .expect("valid per-addon mirror");

        let default_addon_url = origins
            .policy_for_addon("other-addon")
            .resolve("https://dl.liteloader.com/versions", "versions.json")
            .unwrap();
        let custom_addon_url = origins
            .policy_for_addon("custom-addon")
            .resolve("https://dl.liteloader.com/versions", "versions.json")
            .unwrap();

        assert_eq!(
            default_addon_url,
            "https://mirror.example.org/global-ll/versions.json"
        );
        assert_eq!(
            custom_addon_url,
            "https://mirror.example.org/custom-ll/versions.json"
        );
    }
}
