use std::{
    fmt::Debug,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::vanilla::source::{
        VanillaEndpoints, VanillaSource, rewrite_upstream_with_vanilla_fallback,
    },
    families::{
        installer::{
            InstallerArtifact, InstallerInstallStatus, InstallerLaunchVersionRequest,
            ensure_installer_profile_libraries_downloaded, installer_install_status,
            load_persisted_installer_state, prepare_installer_launch_version,
            prepare_installer_state, resolve_installer_processor_runtime,
            run_installer_client_processors, validate_installer_profile_identity,
        },
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
            VersionJsonRootResource,
        },
    },
};

pub trait InstallerFamily: Clone + Copy + Debug + Send + Sync + 'static {
    type Source: Clone + Debug;
    type Endpoints: Clone + Debug + Send + Sync + 'static;

    const FAMILY_NAME: &'static str;

    fn source_endpoints(source: &Self::Source) -> &Self::Endpoints;

    fn installer_artifact<L>(
        source: &Self::Source,
        game_storage: &Storage<L>,
        game_version: &str,
        loader_version: &str,
    ) -> Result<InstallerArtifact>
    where
        L: VersionJsonRootLayout;

    fn profile_identity(install_profile: &ForgeInstallerProfile) -> Result<(String, String)>;

    fn normalize_libraries(
        endpoints: &Self::Endpoints,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>>;

    fn rewrite_upstream(endpoints: &Self::Endpoints, raw_url: &str) -> Result<String>;

    fn default_artifact_url(endpoints: &Self::Endpoints, artifact_path: &str) -> Result<String>;
}

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
pub type PreparedInstallerFamilyLaunchVersion<F, L, VL> =
    PreparedVersionJsonInstance<InstallerFamilyRemoteResolver<F>, L, VL>;
pub type InstallerFamilyInstallStatus = InstallerInstallStatus;

#[derive(Debug, Clone)]
pub struct ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub source: F::Source,
    pub instance: Storage<VL, Storage<L>>,
    pub game_version: String,
    pub loader_version: String,
    pub installer_artifact: InstallerArtifact,
    family: PhantomData<F>,
}

#[derive(Debug, Clone)]
pub struct PreparedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub resolved_version: ResolvedInstallerFamilyVersion<F, L, VL>,
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
    pub launch_version: PreparedInstallerFamilyLaunchVersion<F, L, VL>,
    pub install_status: InstallerFamilyInstallStatus,
}

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

        F::rewrite_upstream(&self.endpoints, raw_url)
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

impl<F, L, VL> ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn new(
        source: F::Source,
        instance: Storage<VL, Storage<L>>,
        game_version: String,
        loader_version: String,
    ) -> Result<Self> {
        let installer_artifact =
            F::installer_artifact(&source, &instance.parent, &game_version, &loader_version)?;

        Ok(Self {
            source,
            instance,
            game_version,
            loader_version,
            installer_artifact,
            family: PhantomData,
        })
    }

    fn validate_profile_identity(&self, install_profile: &ForgeInstallerProfile) -> Result<()> {
        let (profile_game_version, profile_loader_version) = F::profile_identity(install_profile)?;

        validate_installer_profile_identity(
            &self.game_version,
            &self.loader_version,
            &profile_game_version,
            &profile_loader_version,
            F::FAMILY_NAME,
        )
    }
}

impl<F, L, VL> ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn prepare(
        &self,
        downloader: &ElementalDownloader,
        vanilla_source: &VanillaSource,
        remote_resolver: &InstallerFamilyRemoteResolver<F>,
        runtime_executable_path: Option<&Path>,
    ) -> Result<PreparedInstallerFamilyVersion<F, L, VL>> {
        let libraries_root = self
            .instance
            .parent
            .try_get_resource(VersionJsonRootResource::Libraries(None))?;
        let installer_state = prepare_installer_state(
            downloader,
            &self.installer_artifact,
            &self.instance.path,
            &libraries_root,
            F::FAMILY_NAME,
            |install_profile| self.validate_profile_identity(install_profile),
        )
        .await?;

        let endpoints = F::source_endpoints(&self.source);
        let launch_version = prepare_installer_launch_version(InstallerLaunchVersionRequest {
            instance: &self.instance,
            game_version: &self.game_version,
            remote_resolver,
            downloader,
            vanilla_source,
            embedded_version: installer_state.embedded_version.as_ref(),
            normalize_libraries: |libraries| F::normalize_libraries(endpoints, libraries),
            family_name: F::FAMILY_NAME,
        })
        .await?;

        ensure_installer_profile_libraries_downloaded(
            downloader,
            &self.instance,
            &installer_state.install_profile,
            F::FAMILY_NAME,
            |raw_url, artifact_path| remote_resolver.artifact_url(raw_url, artifact_path),
        )
        .await?;

        let processor_operation_name = format!("{} processors", F::FAMILY_NAME);
        let runtime = resolve_installer_processor_runtime(
            &launch_version,
            runtime_executable_path,
            &processor_operation_name,
        )
        .await?;
        run_installer_client_processors(
            &runtime,
            &self.instance,
            &self.installer_artifact,
            &installer_state.install_profile,
            F::FAMILY_NAME,
        )
        .await?;

        let install_status = installer_install_status(
            &self.instance,
            &self.installer_artifact,
            &installer_state.install_profile,
            &launch_version.resolved_version,
            F::FAMILY_NAME,
        )
        .await?;

        Ok(PreparedInstallerFamilyVersion {
            resolved_version: self.clone(),
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
            launch_version,
            install_status,
        })
    }

    pub async fn load(
        source: F::Source,
        remote_resolver: InstallerFamilyRemoteResolver<F>,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedInstallerFamilyVersion<F, L, VL>> {
        let installer_state = load_persisted_installer_state(&instance.path, F::FAMILY_NAME)?;

        let (game_version, loader_version) = F::profile_identity(&installer_state.install_profile)?;
        let resolved_version = ResolvedInstallerFamilyVersion::new(
            source,
            instance.clone(),
            game_version,
            loader_version,
        )?;
        let launch_version = ResolvedInstallerFamilyLaunchVersion::<F, L, VL>::load(
            remote_resolver,
            instance.clone(),
        )?
        .into_prepared()
        .await?;
        let install_status = installer_install_status(
            &resolved_version.instance,
            &resolved_version.installer_artifact,
            &installer_state.install_profile,
            &launch_version.resolved_version,
            F::FAMILY_NAME,
        )
        .await?;

        if !install_status.processors_completed {
            bail!(
                "local {} instance '{}' is not fully prepared: {:?}",
                F::FAMILY_NAME,
                instance
                    .name()
                    .with_context(|| format!("get {} instance name failed", F::FAMILY_NAME))?,
                install_status
            );
        }

        Ok(PreparedInstallerFamilyVersion {
            resolved_version,
            install_profile: installer_state.install_profile,
            embedded_version: installer_state.embedded_version,
            launch_version,
            install_status,
        })
    }
}

impl<F, L, VL> PreparedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn install_profile_path(&self) -> PathBuf {
        crate::families::installer::install_profile_path(
            &self.resolved_version.instance.path,
            F::FAMILY_NAME,
        )
    }

    pub fn embedded_version_path(&self) -> PathBuf {
        crate::families::installer::embedded_version_path(
            &self.resolved_version.instance.path,
            F::FAMILY_NAME,
        )
    }

    pub fn required_java_major_version(&self) -> usize {
        self.launch_version.required_java_major_version()
    }
}
