mod endpoints;
mod install;

use std::path::PathBuf;

use async_trait::async_trait;
use elemental_core::http::{RequestBuilderExt, SwappableClient};
use elemental_driver::families::version_json::GameStorage;
use elemental_infra::downloader::DownloadSession;
use nidhogg::authlib_injector::Artifact;
#[cfg(feature = "authlib-injector-embed")]
use nidhogg::authlib_injector::{EmbeddedArtifact, embedded};
use serde::Deserialize;

use crate::{
    addon::{
        Addon, AddonCatalogQuery, AddonDescriptor, AddonLaunchArgs, AddonLaunchContext,
        AddonPrepareOutput, AddonPrepareRequest, AddonRelease, PatchModeSupport,
    },
    error::{Error, Result},
};

pub use endpoints::{
    AUTHLIB_INJECTOR_ADDON_ID, AUTHLIB_INJECTOR_OFFICIAL_URL, AuthlibInjector,
    AuthlibInjectorEndpoints, AuthlibInjectorSource,
};

#[cfg(feature = "authlib-injector-embed")]
use install::install_embedded_with_reason;
use install::{
    ensure_remote, jar_path_for_instance, jar_path_for_storage, matches_requested_version,
};

#[derive(Debug, Deserialize)]
struct ArtifactsManifest {
    artifacts: Vec<ArtifactSummary>,
}

#[derive(Debug, Deserialize)]
struct ArtifactSummary {
    build_number: u64,
    version: String,
    #[serde(default)]
    release_time: Option<String>,
}

enum ResolvedArtifact {
    Remote(Artifact),
    #[cfg(feature = "authlib-injector-embed")]
    Embedded {
        embedded: EmbeddedArtifact,
        fallback_reason: Option<String>,
    },
}

/// Built-in addon that manages versioned authlib-injector artifacts and attaches
/// `-javaagent` when launching with an external Yggdrasil `auth_server`.
#[derive(Debug, Clone, Default)]
pub struct AuthlibInjectorAddon {
    endpoints: AuthlibInjectorEndpoints,
    client: SwappableClient,
}

impl AuthlibInjectorAddon {
    /// Creates an `AuthlibInjectorAddon` using official endpoints and default HTTP client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the endpoint policy used to fetch manifests and rewrite download URLs.
    pub fn with_endpoints(mut self, endpoints: AuthlibInjectorEndpoints) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Replaces the [`SwappableClient`] used to query authlib-injector metadata.
    pub fn with_http_client(mut self, client: SwappableClient) -> Self {
        self.client = client;
        self
    }

    /// Resolves the absolute cache path for a specific authlib-injector `version`.
    pub fn jar_path_for_version(&self, storage: &GameStorage, version: &str) -> Result<PathBuf> {
        jar_path_for_storage(storage, version)
    }

    /// Prepares the requested `version` (or latest when `None`) via `session` and
    /// returns the verified [`AuthlibInjector`] descriptor.
    pub async fn install(
        &self,
        storage: &GameStorage,
        session: &DownloadSession,
        version: Option<&str>,
    ) -> Result<AuthlibInjector> {
        self.install_with_resolver(session, version, &|resolved_version| {
            self.jar_path_for_version(storage, resolved_version)
        })
        .await
    }

    async fn install_with_resolver(
        &self,
        session: &DownloadSession,
        version: Option<&str>,
        resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
    ) -> Result<AuthlibInjector> {
        match self.resolve_artifact(version).await? {
            ResolvedArtifact::Remote(artifact) => {
                ensure_remote(&self.endpoints, session, artifact, resolve_path).await
            }
            #[cfg(feature = "authlib-injector-embed")]
            ResolvedArtifact::Embedded {
                embedded,
                fallback_reason,
            } => {
                let path = resolve_path(&embedded.artifact().version)?;
                install_embedded_with_reason(&path, embedded, fallback_reason).await
            }
        }
    }

    async fn fetch_artifacts(&self) -> Result<Vec<ArtifactSummary>> {
        let url = self.endpoints.artifacts_url()?;
        let manifest: ArtifactsManifest = self.client.get(url).send_json().await?;
        Ok(manifest.artifacts)
    }

    async fn resolve_artifact(&self, version: Option<&str>) -> Result<ResolvedArtifact> {
        match version {
            None => self.resolve_latest_artifact().await,
            Some(requested) => self.resolve_pinned_artifact(requested).await,
        }
    }

    async fn resolve_latest_artifact(&self) -> Result<ResolvedArtifact> {
        let url = self.endpoints.latest_artifact_url()?;
        match self.client.get(url).send_json::<Artifact>().await {
            Ok(artifact) => Ok(ResolvedArtifact::Remote(artifact)),
            #[cfg(feature = "authlib-injector-embed")]
            Err(error) => Ok(ResolvedArtifact::Embedded {
                embedded: embedded()?,
                fallback_reason: Some(error.to_string()),
            }),
            #[cfg(not(feature = "authlib-injector-embed"))]
            Err(error) => Err(error.into()),
        }
    }

    async fn resolve_pinned_artifact(&self, requested: &str) -> Result<ResolvedArtifact> {
        match self.fetch_pinned_artifact(requested).await {
            Ok(artifact) => Ok(ResolvedArtifact::Remote(artifact)),
            #[cfg(feature = "authlib-injector-embed")]
            Err(Error::HttpRequest(http_error)) => {
                let embedded_artifact = embedded()?;
                let artifact = embedded_artifact.artifact();
                if matches_requested_version(&artifact.version, artifact.build_number, requested) {
                    Ok(ResolvedArtifact::Embedded {
                        embedded: embedded_artifact,
                        fallback_reason: Some(http_error.to_string()),
                    })
                } else {
                    Err(Error::HttpRequest(http_error))
                }
            }
            Err(error) => Err(error),
        }
    }

    async fn fetch_pinned_artifact(&self, requested: &str) -> Result<Artifact> {
        let artifacts = self.fetch_artifacts().await?;
        let summary = artifacts
            .into_iter()
            .find(|item| matches_requested_version(&item.version, item.build_number, requested))
            .ok_or_else(|| Error::AuthlibInjectorVersionNotFound {
                version: requested.to_owned(),
            })?;
        let url = self.endpoints.artifact_url(summary.build_number)?;
        Ok(self.client.get(url).send_json::<Artifact>().await?)
    }
}

#[async_trait]
impl Addon for AuthlibInjectorAddon {
    fn descriptor(&self) -> AddonDescriptor {
        AddonDescriptor::new(AUTHLIB_INJECTOR_ADDON_ID, "authlib-injector")
    }

    fn patch_mode_support(&self) -> PatchModeSupport {
        PatchModeSupport::VirtualOnly
    }

    fn requires_runtime_hook(&self) -> bool {
        true
    }

    async fn available_versions(
        &self,
        _query: &AddonCatalogQuery<'_>,
    ) -> Result<Option<Vec<AddonRelease>>> {
        let artifacts = self.fetch_artifacts().await?;
        Ok(Some(
            artifacts
                .into_iter()
                .map(|entry| AddonRelease::new(entry.version).with_published_at(entry.release_time))
                .collect(),
        ))
    }

    async fn prepare(&self, request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        let injector = self
            .install_with_resolver(request.session, request.addon_version, &|version| {
                jar_path_for_instance(request.instance, version)
            })
            .await?;
        Ok(AddonPrepareOutput {
            version: Some(injector.version().to_owned()),
            metadata_patch: None,
            managed_files: Vec::new(),
        })
    }

    async fn augment_launch(
        &self,
        args: &mut AddonLaunchArgs,
        ctx: &AddonLaunchContext<'_>,
    ) -> Result<()> {
        let Some(auth_server) = &ctx.credential.auth_server else {
            return Ok(());
        };
        let version = ctx
            .record
            .version
            .as_deref()
            .ok_or_else(|| Error::VersionRequired {
                id: AUTHLIB_INJECTOR_ADDON_ID.to_owned(),
            })?;
        let path = jar_path_for_instance(ctx.instance, version)?;
        if !path.is_file() {
            return Err(Error::AuthlibInjectorMissing { path });
        }
        let injector = AuthlibInjector::new(
            path,
            version.to_owned(),
            AuthlibInjectorSource::Remote,
            None,
        );
        args.extra_jvm_arguments
            .extend(injector.jvm_arguments(auth_server));
        Ok(())
    }
}
