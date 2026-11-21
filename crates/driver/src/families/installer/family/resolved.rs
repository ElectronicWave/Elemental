use std::marker::PhantomData;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::runtime::RuntimeResolver;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::downloader::DownloadSession;
use elemental_schema::{forge::ForgeInstallerProfile, mojang::piston::PistonMetaLibraries};

use crate::{
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::{
        installer::{
            InstallerArtifact, installer_has_client_processors, prepare_installer_state,
            profile::persist_installer_launch_version, run_installer_client_processors,
            submit_installer_profile_libraries, validate_installer_profile_identity,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRemoteResolver,
            VersionJsonRootLayout, VersionJsonRootResource,
        },
    },
    launch::{RuntimeOptions, resolve_prepared_version_runtime},
    loader_version::LoaderVersionId,
};

use super::{InstallerFamily, InstallerFamilyRemoteResolver};

#[derive(Debug, Clone)]
pub struct ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub source: F::Source,
    pub instance: Storage<VL, Storage<L>>,
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
    pub installer_artifact: InstallerArtifact,
    family: PhantomData<F>,
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
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
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
}

impl<F, L, VL> ResolvedInstallerFamilyVersion<F, L, VL>
where
    F: InstallerFamily,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn prepare(
        &self,
        session: &DownloadSession,
        vanilla_source: &VanillaSource,
        remote_resolver: &InstallerFamilyRemoteResolver<F>,
        runtime_options: &RuntimeOptions,
        runtime_resolver: &RuntimeResolver,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let ops = InstallerFamilyContext {
            endpoints: F::source_endpoints(&self.source),
            remote_resolver,
        };
        let context = PrepareInstallerContext {
            instance: &self.instance,
            game_version: &self.game_version,
            loader_version: &self.loader_version,
            installer_artifact: &self.installer_artifact,
            session,
            vanilla_source,
            runtime_options,
            runtime_resolver,
        };
        prepare_installer_instance(context, &ops).await
    }
}

trait InstallerFamilyOps: Send + Sync {
    fn family_name(&self) -> &'static str;
    fn remote_resolver(&self) -> &dyn VersionJsonRemoteResolver;
    fn profile_identity(
        &self,
        install_profile: &ForgeInstallerProfile,
    ) -> Result<(String, LoaderVersionId)>;
    fn normalize_libraries(
        &self,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>>;
    fn merge_libraries(
        &self,
        base: Vec<PistonMetaLibraries>,
        embedded: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries>;
    fn artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String>;
}

struct InstallerFamilyContext<'a, F: InstallerFamily> {
    endpoints: &'a F::Endpoints,
    remote_resolver: &'a InstallerFamilyRemoteResolver<F>,
}

impl<F: InstallerFamily> InstallerFamilyOps for InstallerFamilyContext<'_, F> {
    fn family_name(&self) -> &'static str {
        F::FAMILY_NAME
    }

    fn remote_resolver(&self) -> &dyn VersionJsonRemoteResolver {
        self.remote_resolver
    }

    fn profile_identity(
        &self,
        install_profile: &ForgeInstallerProfile,
    ) -> Result<(String, LoaderVersionId)> {
        F::profile_identity(install_profile)
    }

    fn normalize_libraries(
        &self,
        libraries: Vec<PistonMetaLibraries>,
    ) -> Result<Vec<PistonMetaLibraries>> {
        F::normalize_libraries(self.endpoints, libraries)
    }

    fn merge_libraries(
        &self,
        base: Vec<PistonMetaLibraries>,
        embedded: Vec<PistonMetaLibraries>,
    ) -> Vec<PistonMetaLibraries> {
        F::merge_libraries(base, embedded)
    }

    fn artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String> {
        self.remote_resolver.artifact_url(raw_url, artifact_path)
    }
}

struct PrepareInstallerContext<'a, L: VersionJsonRootLayout, VL: VersionJsonInstanceLayout> {
    instance: &'a Storage<VL, Storage<L>>,
    game_version: &'a MinecraftVersionId,
    loader_version: &'a LoaderVersionId,
    installer_artifact: &'a InstallerArtifact,
    session: &'a DownloadSession,
    vanilla_source: &'a VanillaSource,
    runtime_options: &'a RuntimeOptions,
    runtime_resolver: &'a RuntimeResolver,
}

async fn prepare_installer_instance<L, VL>(
    context: PrepareInstallerContext<'_, L, VL>,
    ops: &dyn InstallerFamilyOps,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let family_name = ops.family_name();
    let libraries_root = context
        .instance
        .parent
        .try_get_resource(VersionJsonRootResource::Libraries(None))?;
    let installer_state = prepare_installer_state(
        context.session,
        context.installer_artifact,
        &context.instance.path,
        &libraries_root,
        family_name,
        &|install_profile| {
            let (profile_game_version, profile_loader_version) =
                ops.profile_identity(install_profile)?;
            validate_installer_profile_identity(
                context.game_version.as_str(),
                context.loader_version,
                &profile_game_version,
                &profile_loader_version,
                family_name,
            )
        },
    )
    .await?;

    let resolved_launch = persist_installer_launch_version(
        context.instance,
        context.game_version,
        context.vanilla_source,
        installer_state.embedded_version.as_deref(),
        &|libraries| ops.normalize_libraries(libraries),
        &|base, embedded| ops.merge_libraries(base, embedded),
        family_name,
    )
    .await?;

    resolved_launch
        .submit_install_batches(ops.remote_resolver(), context.session)
        .await?;
    submit_installer_profile_libraries(
        context.session,
        &context.instance.parent,
        &installer_state.install_profile,
        family_name,
        &|raw_url, artifact_path| ops.artifact_url(raw_url, artifact_path),
    )
    .await?;
    context.session.success().await?;

    let launch_version = resolved_launch.finish_preparation().await?;

    if installer_has_client_processors(&installer_state.install_profile) {
        let runtime = resolve_prepared_version_runtime(
            &launch_version,
            context.runtime_options,
            context.runtime_resolver,
        )
        .await?;
        run_installer_client_processors(
            &runtime,
            context.instance,
            context.installer_artifact,
            &installer_state.install_profile,
            family_name,
        )
        .await?;
    }

    Ok(launch_version)
}
