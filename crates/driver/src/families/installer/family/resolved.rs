use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::runtime::RuntimeResolver;
use elemental_core::storage::layout::Layoutable;
use elemental_infra::downloader::DownloadSession;

use crate::{
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::{
        installer::{
            InstallerArtifact, InstallerMavenSource, installer_has_client_processors,
            prepare_installer_state, profile::persist_installer_launch_version,
            run_installer_client_processors, submit_installer_profile_libraries,
            validate_installer_profile_identity,
        },
        version_json::{InstanceStorage, PreparedVersionJsonInstance, VersionJsonRootResource},
    },
    launch::{RuntimeOptions, resolve_prepared_version_runtime},
    loader_version::LoaderVersionId,
};

use super::InstallerFamilyRemoteResolver;

#[derive(Debug, Clone)]
pub struct ResolvedInstallerFamilyVersion {
    pub source: InstallerMavenSource,
    pub instance: InstanceStorage,
    pub game_version: MinecraftVersionId,
    pub loader_version: LoaderVersionId,
    pub installer_artifact: InstallerArtifact,
}

impl ResolvedInstallerFamilyVersion {
    pub fn new(
        source: InstallerMavenSource,
        instance: InstanceStorage,
        game_version: MinecraftVersionId,
        loader_version: LoaderVersionId,
    ) -> Result<Self> {
        let installer_artifact =
            source.installer_artifact(&instance.parent, &game_version, &loader_version)?;

        Ok(Self {
            source,
            instance,
            game_version,
            loader_version,
            installer_artifact,
        })
    }

    pub async fn prepare(
        &self,
        session: &DownloadSession,
        vanilla_source: &VanillaSource,
        remote_resolver: &InstallerFamilyRemoteResolver,
        runtime_options: &RuntimeOptions,
        runtime_resolver: &RuntimeResolver,
    ) -> Result<PreparedVersionJsonInstance> {
        let spec = self.source.spec();
        let family_name = spec.family_name;
        let libraries_root = self
            .instance
            .parent
            .try_get_resource(VersionJsonRootResource::Libraries(None))?;
        let installer_state = prepare_installer_state(
            session,
            &self.installer_artifact,
            &self.instance.path,
            &libraries_root,
            family_name,
            &|install_profile| {
                let (profile_game_version, profile_loader_version) =
                    spec.profile_identity(install_profile)?;
                validate_installer_profile_identity(
                    self.game_version.as_str(),
                    &self.loader_version,
                    &profile_game_version,
                    &profile_loader_version,
                    family_name,
                )
            },
        )
        .await?;

        let resolved_launch = persist_installer_launch_version(
            &self.instance,
            &self.game_version,
            vanilla_source,
            installer_state.embedded_version.as_deref(),
            &|libraries| spec.normalize_libraries(self.source.endpoints(), libraries),
            &|base, embedded| spec.merge_libraries(base, embedded),
            family_name,
        )
        .await?;

        resolved_launch
            .submit_install_batches(remote_resolver, session)
            .await?;
        submit_installer_profile_libraries(
            session,
            &self.instance.parent,
            &installer_state.install_profile,
            family_name,
            &|raw_url, artifact_path| remote_resolver.artifact_url(raw_url, artifact_path),
        )
        .await?;
        session.success().await?;

        let launch_version = resolved_launch.finish_preparation().await?;

        if installer_has_client_processors(&installer_state.install_profile) {
            let runtime = resolve_prepared_version_runtime(
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
                family_name,
            )
            .await?;
        }

        Ok(launch_version)
    }
}
