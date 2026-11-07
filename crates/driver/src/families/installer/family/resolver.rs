use std::marker::PhantomData;

use crate::{
    drivers::vanilla::source::{VanillaEndpoints, rewrite_upstream_with_vanilla_fallback},
    error::Result,
    families::version_json::{
        ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata, VersionJsonRemoteResolver,
    },
};

use super::InstallerFamily;

#[derive(Debug, Clone)]
pub struct InstallerFamilyRemoteResolver<F>
where
    F: InstallerFamily,
{
    vanilla_endpoints: VanillaEndpoints,
    endpoints: F::Endpoints,
    family: PhantomData<F>,
}

pub type ResolvedInstallerFamilyMetadata<F> =
    ResolvedVersionJsonMetadata<InstallerFamilyRemoteResolver<F>>;
pub type ResolvedInstallerFamilyLaunchVersion<F, L, VL> =
    ResolvedVersionJsonInstance<InstallerFamilyRemoteResolver<F>, L, VL>;

impl<F> InstallerFamilyRemoteResolver<F>
where
    F: InstallerFamily,
{
    pub fn new(vanilla_endpoints: VanillaEndpoints, endpoints: F::Endpoints) -> Self {
        Self {
            vanilla_endpoints,
            endpoints,
            family: PhantomData,
        }
    }

    pub fn artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String> {
        if raw_url.trim().is_empty() {
            return F::default_artifact_url(&self.endpoints, artifact_path);
        }

        self.rewrite_upstream(raw_url)
    }
}

impl<F> VersionJsonRemoteResolver for InstallerFamilyRemoteResolver<F>
where
    F: InstallerFamily,
{
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(
            &self.vanilla_endpoints,
            raw_url,
            F::FAMILY_NAME,
            || F::rewrite_upstream(&self.endpoints, raw_url),
        )
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}
