use std::marker::PhantomData;

use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::runtime::RuntimeResolver;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::downloader::DownloadSession;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::{
        installer::{
            InstallerArtifact, InstallerLaunchVersionRequest, installer_has_client_processors,
            persist_installer_launch_version, prepare_installer_state,
            resolve_installer_processor_runtime, run_installer_client_processors,
            submit_installer_profile_libraries, validate_installer_profile_identity,
        },
        version_json::{
            PreparedVersionJsonInstance, VersionJsonInstanceLayout, VersionJsonRootLayout,
            VersionJsonRootResource,
        },
    },
    launch::RuntimeOptions,
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

    fn validate_profile_identity(&self, install_profile: &ForgeInstallerProfile) -> Result<()> {
        let (profile_game_version, profile_loader_version) = F::profile_identity(install_profile)?;

        validate_installer_profile_identity(
            self.game_version.as_str(),
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
        session: &DownloadSession,
        vanilla_source: &VanillaSource,
        remote_resolver: &InstallerFamilyRemoteResolver<F>,
        runtime_options: &RuntimeOptions,
        runtime_resolver: &RuntimeResolver,
    ) -> Result<PreparedVersionJsonInstance<L, VL>> {
        let libraries_root = self
            .instance
            .parent
            .try_get_resource(VersionJsonRootResource::Libraries(None))?;
        let installer_state = prepare_installer_state(
            session,
            &self.installer_artifact,
            &self.instance.path,
            &libraries_root,
            F::FAMILY_NAME,
            |install_profile| self.validate_profile_identity(install_profile),
        )
        .await?;

        let endpoints = F::source_endpoints(&self.source);
        let launch_version = persist_installer_launch_version(InstallerLaunchVersionRequest {
            instance: &self.instance,
            game_version: &self.game_version,
            remote_resolver,
            vanilla_source,
            embedded_version: installer_state.embedded_version.as_ref(),
            normalize_libraries: |libraries| F::normalize_libraries(endpoints, libraries),
            merge_libraries: F::merge_libraries,
            family_name: F::FAMILY_NAME,
        })
        .await?;

        launch_version.submit_install_batches(session).await?;

        submit_installer_profile_libraries(
            session,
            &self.instance,
            &installer_state.install_profile,
            F::FAMILY_NAME,
            |raw_url, artifact_path| remote_resolver.artifact_url(raw_url, artifact_path),
        )
        .await?;

        session.success().await?;

        let launch_version: PreparedVersionJsonInstance<L, VL> =
            launch_version.finish_preparation().await?;

        if installer_has_client_processors(&installer_state.install_profile) {
            let runtime = resolve_installer_processor_runtime(
                &launch_version,
                runtime_options,
                runtime_resolver,
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
        }

        Ok(launch_version)
    }
}
