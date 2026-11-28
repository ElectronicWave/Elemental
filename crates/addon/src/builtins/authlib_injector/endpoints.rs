use std::path::{Path, PathBuf};

use elemental_core::url::{OriginEndpoints, OriginPolicy};

use crate::error::Result;

/// Machine-readable identifier for the built-in authlib-injector addon.
pub const AUTHLIB_INJECTOR_ADDON_ID: &str = "authlib-injector";

/// Canonical upstream base URL for authlib-injector builds.
pub const AUTHLIB_INJECTOR_OFFICIAL_URL: &str = nidhogg::authlib_injector::OFFICIAL_URL;

const KNOWN_ORIGINS: &[&str] = &[AUTHLIB_INJECTOR_OFFICIAL_URL];

/// Endpoint resolver for authlib-injector metadata and artifact downloads.
#[derive(Debug, Clone)]
pub struct AuthlibInjectorEndpoints {
    endpoints: OriginEndpoints,
}

impl Default for AuthlibInjectorEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl AuthlibInjectorEndpoints {
    /// Creates endpoints applying the given `origin_policy`.
    pub fn new(origin_policy: OriginPolicy) -> Self {
        Self {
            endpoints: OriginEndpoints::new(KNOWN_ORIGINS, origin_policy),
        }
    }

    /// Creates endpoints targeting the official authlib-injector origin.
    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    /// Resolves the `artifacts.json` manifest URL listing all releases.
    pub fn artifacts_url(&self) -> Result<String> {
        Ok(self
            .endpoints
            .resolve(AUTHLIB_INJECTOR_OFFICIAL_URL, "artifacts.json")?)
    }

    /// Resolves the `artifact/latest.json` URL describing the newest release.
    pub fn latest_artifact_url(&self) -> Result<String> {
        Ok(self
            .endpoints
            .resolve(AUTHLIB_INJECTOR_OFFICIAL_URL, "artifact/latest.json")?)
    }

    /// Resolves the `artifact/{build_number}.json` URL for a specific build.
    pub fn artifact_url(&self, build_number: u64) -> Result<String> {
        Ok(self.endpoints.resolve(
            AUTHLIB_INJECTOR_OFFICIAL_URL,
            &format!("artifact/{build_number}.json"),
        )?)
    }

    /// Rewrites an upstream download URL according to the configured mirror policy.
    pub fn rewrite_download_url(&self, raw_url: &str) -> Result<String> {
        Ok(self.endpoints.rewrite_known_or_keep(raw_url)?)
    }
}

/// Origin from which a prepared authlib-injector JAR was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthlibInjectorSource {
    /// Downloaded or matched from the remote distribution API.
    Remote,
    /// Materialized from the compile-time embedded snapshot.
    #[cfg(feature = "authlib-injector-embed")]
    Embedded,
}

/// Prepared authlib-injector artifact metadata and resolved JAR path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthlibInjector {
    path: PathBuf,
    version: String,
    source: AuthlibInjectorSource,
    fallback_reason: Option<String>,
}

impl AuthlibInjector {
    pub(super) fn new(
        path: PathBuf,
        version: String,
        source: AuthlibInjectorSource,
        fallback_reason: Option<String>,
    ) -> Self {
        Self {
            path,
            version,
            source,
            fallback_reason,
        }
    }

    /// Absolute path to the prepared `authlib-injector` JAR on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Resolved `authlib-injector` version string.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Where this artifact was obtained from.
    pub fn source(&self) -> AuthlibInjectorSource {
        self.source
    }

    /// Fallback reason if an embedded snapshot replaced a failed remote fetch.
    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }

    /// Returns the JVM arguments required to attach authlib-injector for `auth_server`.
    pub fn jvm_arguments(&self, auth_server: &str) -> [String; 2] {
        [
            format!("-javaagent:{}={auth_server}", self.path.display()),
            "-Dauthlibinjector.side=client".to_owned(),
        ]
    }
}
