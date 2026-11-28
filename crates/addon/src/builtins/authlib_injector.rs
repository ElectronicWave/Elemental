use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use elemental_core::{
    http::{RequestBuilderExt, SwappableClient},
    storage::layout::Layoutable,
    url::{OriginEndpoints, OriginPolicy},
};
use elemental_driver::families::version_json::{
    GameStorage, InstanceStorage, VersionJsonRootResource,
};
use elemental_infra::{
    SafeRelativePath,
    downloader::{DownloadSession, DownloadTask},
};
use nidhogg::authlib_injector::Artifact;
#[cfg(feature = "authlib-injector-embed")]
use nidhogg::authlib_injector::{EmbeddedArtifact, embedded};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    addon::{
        Addon, AddonCatalogQuery, AddonDescriptor, AddonLaunchArgs, AddonLaunchContext,
        AddonPrepareOutput, AddonPrepareRequest, AddonRelease, PatchModeSupport,
    },
    error::{Error, Result},
};

/// Machine-readable identifier for the built-in authlib-injector addon.
pub const AUTHLIB_INJECTOR_ADDON_ID: &str = "authlib-injector";

/// Canonical upstream base URL for authlib-injector builds.
pub const AUTHLIB_INJECTOR_OFFICIAL_URL: &str = nidhogg::authlib_injector::OFFICIAL_URL;

const ARTIFACT_DIRECTORY: &str = "authlib-injector";
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
        let relative = artifact_relative_path(version)?;
        let path = storage.try_get_resource(VersionJsonRootResource::Elemental(Some(relative)))?;
        Ok(std::path::absolute(path)?)
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
                self.ensure_remote(session, artifact, resolve_path).await
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

    async fn ensure_remote(
        &self,
        session: &DownloadSession,
        artifact: Artifact,
        resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
    ) -> Result<AuthlibInjector> {
        let path = resolve_path(&artifact.version)?;
        if existing_artifact_matches(&path, &artifact.checksums.sha256).await? {
            return Ok(AuthlibInjector {
                path,
                version: artifact.version,
                source: AuthlibInjectorSource::Remote,
                fallback_reason: None,
            });
        }

        let download_url = self
            .endpoints
            .rewrite_download_url(&artifact.download_url)?;
        match install_remote(&path, session, &download_url, &artifact.checksums.sha256).await {
            Ok(()) => Ok(AuthlibInjector {
                path,
                version: artifact.version,
                source: AuthlibInjectorSource::Remote,
                fallback_reason: None,
            }),
            Err(error) => install_embedded_fallback(resolve_path, &artifact.version, error).await,
        }
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
            version: Some(injector.version),
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
        let injector = AuthlibInjector {
            path,
            version: version.to_owned(),
            source: AuthlibInjectorSource::Remote,
            fallback_reason: None,
        };
        args.extra_jvm_arguments
            .extend(injector.jvm_arguments(auth_server));
        Ok(())
    }
}

async fn install_remote(
    path: &Path,
    session: &DownloadSession,
    download_url: &str,
    expected_sha256: &str,
) -> Result<()> {
    ensure_parent(path).await?;
    remove_if_present(path).await?;
    session
        .submit(DownloadTask::new(download_url, path))
        .await?;
    verify_file(path, expected_sha256).await
}

async fn verify_file(path: &Path, expected: &str) -> Result<()> {
    let Some(actual) = file_sha256(path).await? else {
        return Err(Error::AuthlibInjectorMissing {
            path: path.to_owned(),
        });
    };
    if actual == expected {
        return Ok(());
    }

    remove_if_present(path).await?;
    Err(Error::AuthlibInjectorChecksum {
        path: path.to_owned(),
        expected: expected.to_owned(),
        actual,
    })
}

fn artifact_relative_path(version: &str) -> Result<SafeRelativePath> {
    Ok(SafeRelativePath::parse(format!(
        "{ARTIFACT_DIRECTORY}/authlib-injector-{version}.jar"
    ))?)
}

fn jar_path_for_instance(instance: &InstanceStorage, version: &str) -> Result<PathBuf> {
    let relative = artifact_relative_path(version)?;
    let path =
        instance.try_get_parent_resource(VersionJsonRootResource::Elemental(Some(relative)))?;
    Ok(std::path::absolute(path)?)
}

#[cfg(feature = "authlib-injector-embed")]
async fn install_embedded_fallback(
    resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
    requested_version: &str,
    error: Error,
) -> Result<AuthlibInjector> {
    let embedded_artifact = embedded()?;
    if embedded_artifact.artifact().version != requested_version {
        return Err(error);
    }
    let path = resolve_path(&embedded_artifact.artifact().version)?;
    install_embedded_with_reason(&path, embedded_artifact, Some(error.to_string())).await
}

#[cfg(not(feature = "authlib-injector-embed"))]
async fn install_embedded_fallback(
    _resolve_path: &(dyn Fn(&str) -> Result<PathBuf> + Sync),
    _requested_version: &str,
    error: Error,
) -> Result<AuthlibInjector> {
    Err(error)
}

#[cfg(feature = "authlib-injector-embed")]
async fn install_embedded_with_reason(
    path: &Path,
    embedded: EmbeddedArtifact,
    fallback_reason: Option<String>,
) -> Result<AuthlibInjector> {
    ensure_parent(path).await?;
    remove_if_present(path).await?;
    tokio::fs::write(path, embedded.jar()).await?;
    let artifact = embedded.artifact();
    verify_file(path, &artifact.checksums.sha256).await?;

    Ok(AuthlibInjector {
        path: path.to_owned(),
        version: artifact.version.clone(),
        source: AuthlibInjectorSource::Embedded,
        fallback_reason,
    })
}

fn matches_requested_version(version: &str, build_number: u64, requested: &str) -> bool {
    version == requested || build_number.to_string() == requested
}

async fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    Ok(())
}

async fn existing_artifact_matches(path: &Path, expected: &str) -> Result<bool> {
    let Some(actual) = file_sha256(path).await? else {
        return Ok(false);
    };
    Ok(actual == expected)
}

async fn file_sha256(path: &Path) -> Result<Option<String>> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(Some(hex::encode(Sha256::digest(bytes)))),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(source.into()),
    }
}

async fn remove_if_present(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(()),
        Err(source) => Err(source.into()),
    }
}
