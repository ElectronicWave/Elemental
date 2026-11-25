use crate::{
    error::Result,
    families::{
        installer::MavenEndpoints,
        version_json::{
            VanillaEndpoints, VersionJsonRemoteResolver, rewrite_upstream_with_vanilla_fallback,
        },
    },
};

#[derive(Debug, Clone)]
pub struct InstallerFamilyRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    endpoints: MavenEndpoints,
}

impl InstallerFamilyRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, endpoints: MavenEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            endpoints,
        }
    }
}

impl VersionJsonRemoteResolver for InstallerFamilyRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(
            &self.vanilla_endpoints,
            raw_url,
            self.endpoints.spec().family_name,
            || self.endpoints.rewrite_upstream(raw_url),
        )
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }

    fn artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String> {
        if raw_url.trim().is_empty() {
            return self.endpoints.artifact_url(artifact_path);
        }

        self.rewrite_upstream(raw_url)
    }
}
