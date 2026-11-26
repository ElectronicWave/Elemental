use async_trait::async_trait;
use elemental_core::minecraft::MinecraftVersionId;
use elemental_infra::downloader::DownloadSession;

use crate::{
    descriptors::DriverDescriptor,
    error::{Error, Result},
    families::{
        installer::{
            InstallerArtifact, InstallerFamilyRemoteResolver, InstallerMavenSource,
            MavenInstallerSpec, installer_install_status, load_persisted_install_profile,
            persist_installer_launch_version, prepare_installer_state,
            validate_installer_profile_identity,
        },
        version_json::{
            InstanceStorage, PreparedVersionJsonInstance, VanillaSource, VersionJsonRootResource,
        },
    },
    inspect::InstalledDriver,
    loader_version::LoaderVersionId,
};

use super::{Driver, InstallerExecutionPlan, InstanceProbe, PrepareRequest, ResolvedDriverPlan};

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
            .try_get_parent_resource(VersionJsonRootResource::Libraries(None))?;
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
            &|libraries| self.source.normalize_libraries(libraries),
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

    pub async fn verify_prepared(
        source: &InstallerMavenSource,
        instance: &InstanceStorage,
        prepared: &PreparedVersionJsonInstance,
    ) -> Result<()> {
        let spec = source.spec();
        let family_name = spec.family_name;
        let install_profile = load_persisted_install_profile(&instance.path, family_name)?;
        let (game_version, loader_version) = spec.profile_identity(&install_profile)?;
        let installer_artifact = source.installer_artifact(
            &instance.parent,
            &MinecraftVersionId::from(game_version),
            &loader_version,
        )?;

        let install_status = installer_install_status(
            instance,
            &installer_artifact,
            &install_profile,
            prepared.install_status.is_ready(),
            family_name,
        )
        .await?;

        if !install_status.processors_completed {
            let instance_name = instance.name().ok_or_else(|| Error::MissingInstanceName {
                family_name: family_name.to_owned(),
                path: instance.path.clone(),
            })?;
            return Err(Error::InstallerNotPrepared {
                family_name: family_name.to_owned(),
                instance: instance_name,
                status: format!("{install_status:?}"),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct InstallerFamilyDriver {
    source: InstallerMavenSource,
    vanilla_source: VanillaSource,
}

impl InstallerFamilyDriver {
    pub fn new(source: InstallerMavenSource) -> Self {
        Self {
            source,
            vanilla_source: VanillaSource::default(),
        }
    }

    pub fn for_spec(spec: &'static MavenInstallerSpec) -> Self {
        Self::new(InstallerMavenSource::official(spec))
    }

    pub fn with_vanilla_source(mut self, vanilla_source: VanillaSource) -> Self {
        self.vanilla_source = vanilla_source;
        self
    }

    pub fn descriptor(&self) -> DriverDescriptor {
        self.source.spec().descriptor()
    }

    fn remote_resolver(&self) -> InstallerFamilyRemoteResolver {
        InstallerFamilyRemoteResolver::new(
            self.vanilla_source.endpoints().clone(),
            self.source.endpoints().clone(),
        )
    }
}

#[async_trait]
impl Driver for InstallerFamilyDriver {
    fn descriptor(&self) -> DriverDescriptor {
        self.source.spec().descriptor()
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledDriver>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };

        Ok(self.source.spec().identity.inspect_installed(metadata))
    }

    async fn resolve(&self, request: &PrepareRequest<'_>) -> Result<ResolvedDriverPlan> {
        let loader_version = request.required_loader_version(self.descriptor())?;
        ResolvedInstallerFamilyVersion::new(
            self.source.clone(),
            request.instance.clone(),
            request.game_version.clone(),
            loader_version.clone(),
        )?
        .resolve(
            request.session,
            &self.vanilla_source,
            self.remote_resolver(),
        )
        .await
    }

    async fn verify_prepared(
        &self,
        instance: &InstanceStorage,
        prepared: &PreparedVersionJsonInstance,
    ) -> Result<()> {
        ResolvedInstallerFamilyVersion::verify_prepared(&self.source, instance, prepared).await
    }
}
