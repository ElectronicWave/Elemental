use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use elemental_core::storage::Storage;
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::{
        forge::source::{ForgeEndpoints, ForgeSource, parse_installer_version},
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
        },
    },
};

pub type ForgeInstallStatus = InstallerInstallStatus;

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
        let installer_state = prepare_installer_state(
            downloader,
            &self.installer_artifact,
            &self.instance.path,
            &self.instance.parent.path.join("libraries"),
            "forge",
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
                normalize_forge_library_urls(libraries, self.source.endpoints())
            },
            family_name: "forge",
        })
        .await?;

        ensure_installer_profile_libraries_downloaded(
            downloader,
            &self.instance,
            &installer_state.install_profile,
            "forge",
            |raw_url, artifact_path| remote_resolver.forge_artifact_url(raw_url, artifact_path),
        )
        .await?;

        let runtime = resolve_installer_processor_runtime(
            &launch_version,
            runtime_executable_path,
            "forge processors",
        )
        .await?;
        run_installer_client_processors(
            &runtime,
            &self.instance,
            &self.installer_artifact,
            &installer_state.install_profile,
            "forge",
        )
        .await?;

        let install_status = installer_install_status(
            &self.instance,
            &self.installer_artifact,
            &installer_state.install_profile,
            &launch_version.resolved_version,
            "forge",
        )
        .await?;

        Ok(PreparedForgeVersion {
            resolved_version: self.clone(),
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
            launch_version,
            install_status,
        })
    }

    pub async fn load(
        source: ForgeSource,
        remote_resolver: ForgeRemoteResolver,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        let installer_state = load_persisted_installer_state(&instance.path, "forge")?;

        let (game_version, loader_version) = profile_identity(&installer_state.install_profile)?;
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
        let install_status = installer_install_status(
            &resolved_version.instance,
            &resolved_version.installer_artifact,
            &installer_state.install_profile,
            &launch_version.resolved_version,
            "forge",
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
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
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
