use elemental_core::minecraft::MinecraftVersionId;
use elemental_core::storage::layout::Layoutable;
use elemental_infra::downloader::DownloadSession;

use crate::{
    driver::{InstallerExecutionPlan, ResolvedDriverPlan},
    drivers::vanilla::source::VanillaSource,
    error::Result,
    families::{
        installer::{
            InstallerArtifact, InstallerMavenSource, prepare_installer_state,
            profile::persist_installer_launch_version, validate_installer_profile_identity,
        },
        version_json::{InstanceStorage, VersionJsonRootResource},
    },
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

    pub async fn resolve(
        &self,
        session: &DownloadSession,
        vanilla_source: &VanillaSource,
        remote_resolver: InstallerFamilyRemoteResolver,
    ) -> Result<ResolvedDriverPlan> {
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

        Ok(ResolvedDriverPlan {
            instance: resolved_launch,
            remote_resolver: Box::new(remote_resolver),
            download_status: None,
            installer: Some(InstallerExecutionPlan {
                family_name,
                installer_artifact: self.installer_artifact.clone(),
                install_profile: installer_state.install_profile,
            }),
        })
    }
}
