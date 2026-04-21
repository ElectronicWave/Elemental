use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::{
        neoforge::source::{NeoForgeEndpoints, NeoForgeSource},
        shared::rewrite_upstream_with_vanilla_fallback,
        vanilla::source::{VanillaEndpoints, VanillaSource},
    },
    families::{
        installer::{
            InstallerArtifact, InstallerInstallStatus, InstallerLaunchVersionRequest,
            embedded_version_path as installer_embedded_version_path,
            ensure_installer_profile_libraries_downloaded,
            install_profile_path as installer_install_profile_path, installer_install_status,
            load_persisted_installer_state, normalize_library_urls,
            prepare_installer_launch_version, prepare_installer_state,
            profile_game_and_raw_loader_version, resolve_installer_processor_runtime,
            run_installer_client_processors, validate_installer_profile_identity,
        },
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
            VersionJsonRootResource,
        },
    },
};

pub type NeoForgeInstallStatus = InstallerInstallStatus;

#[derive(Debug, Clone)]
pub struct NeoForgeRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    neoforge_endpoints: NeoForgeEndpoints,
}

pub type ResolvedNeoForgeMetadata = ResolvedVersionJsonMetadata<NeoForgeRemoteResolver>;
pub type ResolvedNeoForgeLaunchVersion<L, VL> =
    ResolvedVersionJsonInstance<NeoForgeRemoteResolver, L, VL>;
pub type PreparedNeoForgeLaunchVersion<L, VL> =
    PreparedVersionJsonInstance<NeoForgeRemoteResolver, L, VL>;

#[derive(Debug, Clone)]
pub struct ResolvedNeoForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub source: NeoForgeSource,
    pub instance: Storage<VL, Storage<L>>,
    pub game_version: String,
    pub loader_version: String,
    pub installer_artifact: InstallerArtifact,
}

#[derive(Debug, Clone)]
pub struct PreparedNeoForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub resolved_version: ResolvedNeoForgeVersion<L, VL>,
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
    pub launch_version: PreparedNeoForgeLaunchVersion<L, VL>,
    pub install_status: NeoForgeInstallStatus,
}

impl NeoForgeRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, neoforge_endpoints: NeoForgeEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            neoforge_endpoints,
        }
    }

    pub fn neoforge_artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String> {
        if raw_url.trim().is_empty() {
            return self.neoforge_endpoints.maven_artifact_url(artifact_path);
        }

        self.neoforge_endpoints.rewrite_upstream(raw_url)
    }
}

impl VersionJsonRemoteResolver for NeoForgeRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        rewrite_upstream_with_vanilla_fallback(&self.vanilla_endpoints, raw_url, "neoforge", || {
            self.neoforge_endpoints.rewrite_upstream(raw_url)
        })
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}

impl<L, VL> ResolvedNeoForgeVersion<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn prepare(
        &self,
        downloader: &ElementalDownloader,
        vanilla_source: &VanillaSource,
        remote_resolver: &NeoForgeRemoteResolver,
        runtime_executable_path: Option<&Path>,
    ) -> Result<PreparedNeoForgeVersion<L, VL>> {
        let libraries_root = self
            .instance
            .parent
            .try_get_resource(VersionJsonRootResource::Libraries(None))?;
        let installer_state = prepare_installer_state(
            downloader,
            &self.installer_artifact,
            &self.instance.path,
            &libraries_root,
            "neoforge",
            |install_profile| validate_profile_identity(self, install_profile),
        )
        .await?;

        let launch_version = prepare_installer_launch_version(InstallerLaunchVersionRequest {
            instance: &self.instance,
            game_version: &self.game_version,
            remote_resolver,
            downloader,
            vanilla_source,
            embedded_version: installer_state.embedded_version.as_ref(),
            normalize_libraries: |libraries| {
                normalize_neoforge_library_urls(libraries, self.source.endpoints())
            },
            family_name: "neoforge",
        })
        .await?;

        ensure_installer_profile_libraries_downloaded(
            downloader,
            &self.instance,
            &installer_state.install_profile,
            "neoforge",
            |raw_url, artifact_path| remote_resolver.neoforge_artifact_url(raw_url, artifact_path),
        )
        .await?;

        let runtime = resolve_installer_processor_runtime(
            &launch_version,
            runtime_executable_path,
            "neoforge processors",
        )
        .await?;
        run_installer_client_processors(
            &runtime,
            &self.instance,
            &self.installer_artifact,
            &installer_state.install_profile,
            "neoforge",
        )
        .await?;

        let install_status = installer_install_status(
            &self.instance,
            &self.installer_artifact,
            &installer_state.install_profile,
            &launch_version.resolved_version,
            "neoforge",
        )
        .await?;

        Ok(PreparedNeoForgeVersion {
            resolved_version: self.clone(),
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
            launch_version,
            install_status,
        })
    }

    pub async fn load(
        source: NeoForgeSource,
        remote_resolver: NeoForgeRemoteResolver,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedNeoForgeVersion<L, VL>> {
        let installer_state = load_persisted_installer_state(&instance.path, "neoforge")?;

        let (game_version, loader_version) = profile_identity(&installer_state.install_profile)?;
        let installer_artifact =
            source.installer_artifact(&instance.parent, &game_version, &loader_version)?;
        let resolved_version = ResolvedNeoForgeVersion {
            source,
            instance: instance.clone(),
            game_version,
            loader_version,
            installer_artifact,
        };
        let launch_version =
            ResolvedNeoForgeLaunchVersion::load(remote_resolver, instance.clone())?
                .into_prepared()
                .await?;
        let install_status = installer_install_status(
            &resolved_version.instance,
            &resolved_version.installer_artifact,
            &installer_state.install_profile,
            &launch_version.resolved_version,
            "neoforge",
        )
        .await?;

        if !install_status.processors_completed {
            bail!(
                "local neoforge instance '{}' is not fully prepared: {:?}",
                instance
                    .name()
                    .context("get neoforge instance name failed")?,
                install_status
            );
        }

        Ok(PreparedNeoForgeVersion {
            resolved_version,
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
            launch_version,
            install_status,
        })
    }
}

impl<L, VL> PreparedNeoForgeVersion<L, VL>
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

fn install_profile_path(instance_root: &Path) -> PathBuf {
    installer_install_profile_path(instance_root, "neoforge")
}

fn embedded_version_path(instance_root: &Path) -> PathBuf {
    installer_embedded_version_path(instance_root, "neoforge")
}

fn profile_identity(install_profile: &ForgeInstallerProfile) -> Result<(String, String)> {
    profile_game_and_raw_loader_version(install_profile, "neoforge", "neoforge")
}

fn validate_profile_identity<L, VL>(
    resolved_version: &ResolvedNeoForgeVersion<L, VL>,
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
        "neoforge",
    )
}

fn normalize_neoforge_library_urls(
    libraries: Vec<PistonMetaLibraries>,
    neoforge_endpoints: &NeoForgeEndpoints,
) -> Result<Vec<PistonMetaLibraries>> {
    normalize_library_urls(libraries, |artifact_path| {
        neoforge_endpoints.maven_artifact_url(artifact_path)
    })
}
