use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::{
        forge::{
            processor::{
                client_processors_ready, ensure_profile_libraries_downloaded, run_client_processors,
            },
            source::{ForgeEndpoints, ForgeSource, parse_installer_version},
        },
        vanilla::source::{VanillaEndpoints, VanillaSource},
    },
    families::{
        installer::{
            InstallerArchive, InstallerArtifact, InstallerLaunchVersionRequest,
            embedded_version_path as installer_embedded_version_path,
            ensure_installer_artifact_downloaded,
            install_profile_path as installer_install_profile_path, normalize_library_urls,
            persist_embedded_version, persist_install_profile, prepare_installer_launch_version,
            profile_game_and_raw_loader_version, profile_libraries_ready, read_embedded_version,
            validate_installer_profile_identity,
        },
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
        },
    },
    runtime::resolve_runtime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeInstallStatus {
    pub installer_downloaded: bool,
    pub install_profile_persisted: bool,
    pub embedded_version_persisted: bool,
    pub profile_libraries_ready: bool,
    pub processors_completed: bool,
    pub launch_version_ready: bool,
}

#[derive(Debug, Clone)]
pub struct ForgeRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    forge_endpoints: ForgeEndpoints,
}

pub type ResolvedForgeMetadata = ResolvedVersionJsonMetadata<ForgeRemoteResolver>;
pub type ResolvedForgeLaunchVersion<L, VL> =
    ResolvedVersionJsonInstance<ForgeRemoteResolver, L, VL>;
pub type PreparedForgeLaunchVersion<L, VL> =
    PreparedVersionJsonInstance<ForgeRemoteResolver, L, VL>;

#[derive(Debug, Clone)]
pub struct ResolvedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub source: ForgeSource,
    pub instance: Storage<VL, Storage<L>>,
    pub game_version: String,
    pub loader_version: String,
    pub installer_artifact: InstallerArtifact,
}

#[derive(Debug, Clone)]
pub struct PreparedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub resolved_version: ResolvedForgeVersion<L, VL>,
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
    pub launch_version: PreparedForgeLaunchVersion<L, VL>,
    pub install_status: ForgeInstallStatus,
}

impl ForgeRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, forge_endpoints: ForgeEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            forge_endpoints,
        }
    }

    pub fn forge_artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String> {
        if raw_url.trim().is_empty() {
            return self.forge_endpoints.maven_artifact_url(artifact_path);
        }

        self.forge_endpoints.rewrite_upstream(raw_url)
    }
}

impl VersionJsonRemoteResolver for ForgeRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Ok(rewritten) = self.vanilla_endpoints.rewrite_upstream(raw_url) {
            return Ok(rewritten);
        }

        self.forge_endpoints
            .rewrite_upstream(raw_url)
            .with_context(|| format!("rewrite forge upstream url failed for '{raw_url}'"))
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}

impl<L, VL> ResolvedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn prepare(
        &self,
        downloader: &ElementalDownloader,
        vanilla_source: &VanillaSource,
        remote_resolver: &ForgeRemoteResolver,
        runtime_executable_path: Option<&Path>,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        ensure_installer_artifact_downloaded(downloader, &self.installer_artifact, "forge").await?;

        let archive = InstallerArchive::new(self.installer_artifact.path.clone());
        let install_profile = archive
            .read_json::<ForgeInstallerProfile>("install_profile.json")
            .context("read forge install_profile.json from installer failed")?;
        validate_profile_identity(self, &install_profile)?;
        let embedded_version = read_embedded_version(&archive, &install_profile, "forge")?;

        persist_install_profile(&self.instance.path, &install_profile, "forge").await?;
        persist_embedded_version(&self.instance.path, embedded_version.as_ref(), "forge").await?;
        archive
            .extract_maven_artifacts(&self.instance.parent.path.join("libraries"))
            .context("extract forge embedded maven artifacts failed")?;

        let launch_version = prepare_installer_launch_version(InstallerLaunchVersionRequest {
            instance: &self.instance,
            game_version: &self.game_version,
            remote_resolver,
            downloader,
            vanilla_source,
            embedded_version: embedded_version.as_ref(),
            normalize_libraries: |libraries| {
                normalize_forge_library_urls(libraries, self.source.endpoints())
            },
            family_name: "forge",
        })
        .await?;

        ensure_profile_libraries_downloaded(
            downloader,
            remote_resolver,
            &self.instance,
            &install_profile,
        )
        .await?;

        let runtime = processor_runtime(&launch_version, runtime_executable_path).await?;
        run_client_processors(
            &runtime,
            &self.instance,
            &self.installer_artifact,
            &install_profile,
        )
        .await?;

        let install_status =
            install_status(self, &install_profile, &launch_version.resolved_version).await?;

        Ok(PreparedForgeVersion {
            resolved_version: self.clone(),
            install_profile,
            embedded_version,
            launch_version,
            install_status,
        })
    }

    pub async fn load(
        source: ForgeSource,
        remote_resolver: ForgeRemoteResolver,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        let install_profile_path = install_profile_path(&instance.path);
        let install_profile = serde_json::from_reader(std::fs::File::open(&install_profile_path)?)
            .with_context(|| {
                format!(
                    "read persisted forge install profile failed: {}",
                    install_profile_path.display()
                )
            })?;

        let embedded_version_path = embedded_version_path(&instance.path);
        let embedded_version = if embedded_version_path.exists() {
            Some(
                serde_json::from_reader(std::fs::File::open(&embedded_version_path)?)
                    .with_context(|| {
                        format!(
                            "read persisted forge embedded version failed: {}",
                            embedded_version_path.display()
                        )
                    })?,
            )
        } else {
            None
        };

        let (game_version, loader_version) = profile_identity(&install_profile)?;
        let installer_artifact =
            source.installer_artifact(&instance.parent, &game_version, &loader_version)?;
        let resolved_version = ResolvedForgeVersion {
            source,
            instance: instance.clone(),
            game_version,
            loader_version,
            installer_artifact,
        };
        let launch_version = ResolvedForgeLaunchVersion::load(remote_resolver, instance.clone())?
            .into_prepared()
            .await?;
        let install_status = install_status(
            &resolved_version,
            &install_profile,
            &launch_version.resolved_version,
        )
        .await?;

        if !install_status.processors_completed {
            bail!(
                "local forge instance '{}' is not fully prepared: {:?}",
                instance.name().context("get forge instance name failed")?,
                install_status
            );
        }

        Ok(PreparedForgeVersion {
            resolved_version,
            install_profile,
            embedded_version,
            launch_version,
            install_status,
        })
    }
}

impl<L, VL> PreparedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn install_profile_path(&self) -> PathBuf {
        install_profile_path(&self.resolved_version.instance.path)
    }

    pub fn embedded_version_path(&self) -> PathBuf {
        embedded_version_path(&self.resolved_version.instance.path)
    }

    pub fn required_java_major_version(&self) -> usize {
        self.launch_version.required_java_major_version()
    }
}

async fn processor_runtime<L, VL>(
    launch_version: &PreparedForgeLaunchVersion<L, VL>,
    runtime_executable_path: Option<&Path>,
) -> Result<Distribution>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version = launch_version.required_java_major_version();
    resolve_runtime(
        required_major_version,
        runtime_executable_path,
        "forge processors",
    )
    .await
}

async fn install_status<L, VL>(
    resolved_version: &ResolvedForgeVersion<L, VL>,
    install_profile: &ForgeInstallerProfile,
    launch_version: &ResolvedForgeLaunchVersion<L, VL>,
) -> Result<ForgeInstallStatus>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    Ok(ForgeInstallStatus {
        installer_downloaded: resolved_version.installer_artifact.path.exists(),
        install_profile_persisted: install_profile_path(&resolved_version.instance.path).exists(),
        embedded_version_persisted: embedded_version_path(&resolved_version.instance.path).exists(),
        profile_libraries_ready: profile_libraries_ready(
            &resolved_version.instance,
            install_profile,
        )?,
        processors_completed: client_processors_ready(
            &resolved_version.instance,
            &resolved_version.installer_artifact,
            install_profile,
        )?,
        launch_version_ready: launch_version.status().await?.is_ready(),
    })
}

fn install_profile_path(instance_root: &Path) -> PathBuf {
    installer_install_profile_path(instance_root, "forge")
}

fn embedded_version_path(instance_root: &Path) -> PathBuf {
    installer_embedded_version_path(instance_root, "forge")
}

fn profile_identity(install_profile: &ForgeInstallerProfile) -> Result<(String, String)> {
    let (game_version, raw_version) =
        profile_game_and_raw_loader_version(install_profile, "forge", "forge")?;
    let (_, loader_version) = parse_installer_version(&raw_version)?;
    Ok((game_version, loader_version))
}

fn validate_profile_identity<L, VL>(
    resolved_version: &ResolvedForgeVersion<L, VL>,
    install_profile: &ForgeInstallerProfile,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let (profile_game_version, profile_loader_version) = profile_identity(install_profile)?;

    validate_installer_profile_identity(
        &resolved_version.game_version,
        &resolved_version.loader_version,
        &profile_game_version,
        &profile_loader_version,
        "forge",
    )
}

fn normalize_forge_library_urls(
    libraries: Vec<PistonMetaLibraries>,
    forge_endpoints: &ForgeEndpoints,
) -> Result<Vec<PistonMetaLibraries>> {
    normalize_library_urls(libraries, |artifact_path| {
        forge_endpoints.maven_artifact_url(artifact_path)
    })
}
