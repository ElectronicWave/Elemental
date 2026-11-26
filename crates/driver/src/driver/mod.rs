mod detect;
mod installer;
mod profiled;

use async_trait::async_trait;
use elemental_core::{minecraft::MinecraftVersionId, runtime::RuntimeResolver};
use elemental_infra::downloader::DownloadSession;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::error::{Error, Result};
use crate::families::{
    installer::{
        InstallerArtifact, installer_has_client_processors, run_installer_client_processors,
        submit_installer_profile_libraries,
    },
    version_json::{
        InstanceStorage, PreparedVersionJsonInstance, ResolvedVersionJsonInstance,
        VersionJsonDownloadStatus, VersionJsonRemoteResolver,
    },
};
use crate::launch::{RuntimeOptions, resolve_prepared_version_runtime};
use crate::loader_version::LoaderVersionId;

pub use crate::descriptors::DriverDescriptor;
pub use crate::inspect::InstalledDriver;
pub use detect::{InstalledInstance, InstanceDetectionFailure, InstanceProbe, inspect_instances};
pub use installer::{InstallerFamilyDriver, ResolvedInstallerFamilyVersion};
pub use profiled::{
    ProfiledVersionJsonDriver, ProfiledVersionJsonFamily, ProfiledVersionJsonFamilyExt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareStage {
    ResolvingMetadata,
    Downloading,
    ExtractingNatives,
    RunningProcessor {
        family_name: String,
        index: usize,
        total: usize,
        jar: String,
    },
    Ready,
}

#[derive(Debug, Clone)]
pub struct InstallerExecutionPlan {
    pub family_name: &'static str,
    pub installer_artifact: InstallerArtifact,
    pub install_profile: ForgeInstallerProfile,
}

#[derive(Debug)]
pub struct ResolvedDriverPlan {
    pub instance: ResolvedVersionJsonInstance,
    pub remote_resolver: Box<dyn VersionJsonRemoteResolver>,
    pub download_status: Option<VersionJsonDownloadStatus>,
    pub installer: Option<InstallerExecutionPlan>,
}

pub struct PrepareRequest<'a> {
    pub session: &'a DownloadSession,
    pub instance: &'a InstanceStorage,
    pub game_version: &'a MinecraftVersionId,
    pub loader_version: Option<&'a LoaderVersionId>,
    pub runtime_options: &'a RuntimeOptions,
    pub runtime_resolver: &'a RuntimeResolver,
    pub on_stage: Option<&'a (dyn Fn(PrepareStage) + Send + Sync)>,
}

impl<'a> PrepareRequest<'a> {
    pub fn required_loader_version(&self, driver: DriverDescriptor) -> Result<&'a LoaderVersionId> {
        self.loader_version
            .ok_or_else(|| Error::LoaderVersionRequired {
                driver: driver.id.to_owned(),
            })
    }

    pub fn emit(&self, stage: PrepareStage) {
        if let Some(observer) = self.on_stage {
            observer(stage);
        }
    }
}

#[async_trait]
pub trait Driver: Send + Sync {
    fn descriptor(&self) -> DriverDescriptor;

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>>;

    /// Resolves metadata and download sources needed to install or refresh an instance.
    async fn resolve(&self, request: &PrepareRequest<'_>) -> Result<ResolvedDriverPlan>;

    /// Verifies family-specific persisted state when loading an existing instance.
    async fn verify_prepared(
        &self,
        _instance: &InstanceStorage,
        _prepared: &PreparedVersionJsonInstance,
    ) -> Result<()> {
        Ok(())
    }

    /// Prepare a version for launch and return the common version JSON view.
    ///
    /// The result contains everything launch construction needs; family-specific
    /// installation state is not part of the public result.
    async fn prepare(&self, request: &PrepareRequest<'_>) -> Result<PreparedVersionJsonInstance> {
        request.emit(PrepareStage::ResolvingMetadata);
        let plan = self.resolve(request).await?;
        let resolver = plan.remote_resolver.as_ref();

        let download_status = match plan.download_status {
            Some(status) if plan.installer.is_none() && status.is_downloaded() => status,
            _ => {
                request.emit(PrepareStage::Downloading);
                plan.instance
                    .submit_install_batches(resolver, request.session)
                    .await?;
                if let Some(installer) = &plan.installer {
                    submit_installer_profile_libraries(
                        request.session,
                        &request.instance.parent,
                        &installer.install_profile,
                        installer.family_name,
                        &|raw_url, artifact_path| resolver.artifact_url(raw_url, artifact_path),
                    )
                    .await?;
                }
                request.session.success().await?;
                plan.instance.download_status()?
            }
        };

        request.emit(PrepareStage::ExtractingNatives);
        let prepared = plan
            .instance
            .finish_preparation_with_download_status(download_status)
            .await?;

        if let Some(installer) = &plan.installer
            && installer_has_client_processors(&installer.install_profile)
        {
            let runtime = resolve_prepared_version_runtime(
                &prepared,
                request.runtime_options,
                request.runtime_resolver,
            )
            .await?;
            let on_processor = |index: usize, total: usize, jar: &str| {
                request.emit(PrepareStage::RunningProcessor {
                    family_name: installer.family_name.to_owned(),
                    index,
                    total,
                    jar: jar.to_owned(),
                });
            };
            run_installer_client_processors(
                &runtime,
                request.instance,
                &installer.installer_artifact,
                &installer.install_profile,
                installer.family_name,
                request
                    .on_stage
                    .map(|_| &on_processor as &(dyn Fn(usize, usize, &str) + Send + Sync)),
            )
            .await?;
        }

        request.emit(PrepareStage::Ready);
        Ok(prepared)
    }

    /// Load a prepared version and return the common version JSON view.
    ///
    /// The result contains everything launch construction needs; family-specific
    /// installation state is not part of the public result.
    async fn load_prepared(
        &self,
        instance: &InstanceStorage,
    ) -> Result<PreparedVersionJsonInstance> {
        let prepared = ResolvedVersionJsonInstance::load(instance.clone())?
            .finish_preparation()
            .await?;
        self.verify_prepared(instance, &prepared).await?;
        Ok(prepared)
    }
}
