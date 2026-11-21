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
            InstallerArtifact, InstallerPersistedState, installer_has_client_processors,
            processor::libraries::submit_installer_profile_libraries_dyn,
            profile::{
                install::prepare_installer_state_dyn, launch::persist_installer_launch_metadata,
            },
            run_installer_client_processors, validate_installer_profile_identity,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
            VersionJsonRootResource,
            prepared::{
                instance::{finish_instance_preparation, submit_instance_install_batches},
                readiness::check_download_status,
            },
            remote::DynRemoteResolver,
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
        let endpoints = F::source_endpoints(&self.source);
        prepare_installer_instance(
            &self.instance,
            &self.game_version,
            &self.loader_version,
            &self.installer_artifact,
            F::FAMILY_NAME,
            session,
            vanilla_source,
            remote_resolver,
            F::profile_identity,
            &|libraries| F::normalize_libraries(endpoints, libraries),
            &F::merge_libraries,
            &|raw_url, artifact_path| remote_resolver.artifact_url(raw_url, artifact_path),
            runtime_options,
            runtime_resolver,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
async fn prepare_installer_instance<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    game_version: &MinecraftVersionId,
    loader_version: &LoaderVersionId,
    installer_artifact: &InstallerArtifact,
    family_name: &str,
    session: &DownloadSession,
    vanilla_source: &VanillaSource,
    remote_resolver: &dyn DynRemoteResolver,
    profile_identity: fn(&ForgeInstallerProfile) -> Result<(String, LoaderVersionId)>,
    normalize_libraries: &(
         dyn Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>> + Send + Sync
     ),
    merge_libraries: &(
         dyn Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>
             + Send
             + Sync
     ),
    artifact_url: &(dyn Fn(&str, &str) -> Result<String> + Send + Sync),
    runtime_options: &RuntimeOptions,
    runtime_resolver: &RuntimeResolver,
) -> Result<PreparedVersionJsonInstance<L, VL>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let libraries_root = instance
        .parent
        .try_get_resource(VersionJsonRootResource::Libraries(None))?;
    let installer_state: InstallerPersistedState = prepare_installer_state_dyn(
        session,
        installer_artifact,
        &instance.path,
        &libraries_root,
        family_name,
        &|install_profile| {
            let (profile_game_version, profile_loader_version) = profile_identity(install_profile)?;
            validate_installer_profile_identity(
                game_version.as_str(),
                loader_version,
                &profile_game_version,
                &profile_loader_version,
                family_name,
            )
        },
    )
    .await?;

    let (metadata, asset_index_objects) = persist_installer_launch_metadata(
        instance,
        game_version,
        vanilla_source,
        installer_state.embedded_version.as_ref(),
        normalize_libraries,
        merge_libraries,
        family_name,
    )
    .await?;

    submit_instance_install_batches(
        remote_resolver,
        instance,
        &metadata,
        &asset_index_objects,
        session,
    )
    .await?;
    submit_installer_profile_libraries_dyn(
        session,
        instance,
        &installer_state.install_profile,
        family_name,
        artifact_url,
    )
    .await?;
    session.success().await?;
    let download_status = check_download_status(instance, &metadata, &asset_index_objects)?;
    let launch_version =
        finish_instance_preparation(instance.clone(), metadata, download_status).await?;

    if installer_has_client_processors(&installer_state.install_profile) {
        let runtime =
            resolve_prepared_version_runtime(&launch_version, runtime_options, runtime_resolver)
                .await?;
        run_installer_client_processors(
            &runtime,
            instance,
            installer_artifact,
            &installer_state.install_profile,
            family_name,
        )
        .await?;
    }

    Ok(launch_version)
}
