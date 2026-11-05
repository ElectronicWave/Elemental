use std::{future::Future, path::PathBuf};

use elemental_core::{
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layoutable},
};
use elemental_infra::downloader::{DownloadBatch, DownloadSession, DownloadTask};
use elemental_schema::mojang::piston::{
    PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaLibraries,
    PistonMetaLibrariesDownloadsArtifact,
};

use crate::error::{Error, Result};

use crate::families::version_json::{
    PistonMetaLibrariesExt, VersionJsonGameStorageExt, VersionJsonInstanceLayout,
    VersionJsonInstanceResource, VersionJsonRootLayout, VersionJsonRootResource,
    VersionJsonRuleContext, VersionJsonVersionStorageExt, remote::VersionJsonRemoteResolver,
};

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonMetadata<R: VersionJsonRemoteResolver> {
    remote_resolver: R,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionJsonInstallStatus {
    pub metadata_persisted: bool,
    pub asset_index_persisted: bool,
    pub version_artifacts_ready: bool,
    pub assets_ready: bool,
    pub natives_extracted: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    remote_resolver: R,
    pub version: Storage<VL, Storage<L>>,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct PreparedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub resolved_version: ResolvedVersionJsonInstance<R, L, VL>,
    pub install_status: VersionJsonInstallStatus,
}

pub struct LaunchedVersionJsonInstance<
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    pub prepared_version: PreparedVersionJsonInstance<R, L, VL>,
    pub runtime: Distribution,
    pub child: tokio::process::Child,
}

pub struct VersionJsonInstallPlanner<
    'a,
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
> {
    version: &'a Storage<VL, Storage<L>>,
    metadata: &'a PistonMetaData,
    asset_index_objects: &'a PistonMetaAssetIndexObjects,
    remote_resolver: R,
    rule_context: VersionJsonRuleContext,
}

impl VersionJsonInstallStatus {
    pub fn is_persisted(&self) -> bool {
        self.metadata_persisted && self.asset_index_persisted
    }

    pub fn is_downloaded(&self) -> bool {
        self.is_persisted() && self.version_artifacts_ready && self.assets_ready
    }

    pub fn is_ready(&self) -> bool {
        self.is_downloaded() && self.natives_extracted
    }
}

impl<R: VersionJsonRemoteResolver> ResolvedVersionJsonMetadata<R> {
    pub fn new(
        remote_resolver: R,
        metadata: PistonMetaData,
        asset_index_objects: PistonMetaAssetIndexObjects,
    ) -> Self {
        Self {
            remote_resolver,
            metadata,
            asset_index_objects,
        }
    }

    pub async fn persist<
        L: VersionJsonRootLayout + Clone,
        VL: VersionJsonInstanceLayout + Clone,
    >(
        self,
        instance: &Storage<VL, Storage<L>>,
    ) -> Result<ResolvedVersionJsonInstance<R, L, VL>> {
        instance.write_metadata(&self.metadata).await?;
        instance
            .parent
            .write_asset_index(
                self.metadata.asset_index.id.clone(),
                &self.asset_index_objects,
            )
            .await?;

        Ok(ResolvedVersionJsonInstance {
            remote_resolver: self.remote_resolver,
            version: instance.clone(),
            metadata: self.metadata,
            asset_index_objects: self.asset_index_objects,
        })
    }
}

pub async fn prepare_version_json<R, L, VL, ResolveFn, ResolveFut>(
    session: &DownloadSession,
    resolve_or_load: ResolveFn,
) -> Result<PreparedVersionJsonInstance<R, L, VL>>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    ResolveFn: FnOnce() -> ResolveFut,
    ResolveFut: Future<Output = Result<ResolvedVersionJsonInstance<R, L, VL>>>,
{
    let resolved = resolve_or_load().await?;
    resolved.prepare(session).await
}

pub async fn persist_version_json<R, L, VL, ResolveFn, ResolveFut>(
    instance: &Storage<VL, Storage<L>>,
    resolve_metadata: ResolveFn,
) -> Result<ResolvedVersionJsonInstance<R, L, VL>>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    ResolveFn: FnOnce() -> ResolveFut,
    ResolveFut: Future<Output = Result<ResolvedVersionJsonMetadata<R>>>,
{
    resolve_metadata().await?.persist(instance).await
}

pub async fn load_prepared_version_json<R, L, VL>(
    remote_resolver: R,
    instance: &Storage<VL, Storage<L>>,
) -> Result<PreparedVersionJsonInstance<R, L, VL>>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    ResolvedVersionJsonInstance::load(remote_resolver, instance.clone())?
        .into_prepared()
        .await
}

impl<R, L, VL> ResolvedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn load(remote_resolver: R, version: Storage<VL, Storage<L>>) -> Result<Self> {
        let metadata = version.metadata()?;
        let asset_index_objects = version
            .parent
            .asset_index_objects(metadata.asset_index.id.as_str())?;

        Ok(Self {
            remote_resolver,
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub async fn into_prepared(self) -> Result<PreparedVersionJsonInstance<R, L, VL>> {
        let status = self.status().await?;
        if !status.is_ready() {
            let version_name = self
                .version
                .name()
                .ok_or_else(|| Error::MissingVersionName {
                    path: self.version.path.clone(),
                })?;
            return Err(Error::VersionNotPrepared {
                version: version_name,
                status: format!("{status:?}"),
            });
        }

        Ok(PreparedVersionJsonInstance {
            install_status: status,
            resolved_version: self,
        })
    }

    pub fn planner<'a>(&'a self) -> VersionJsonInstallPlanner<'a, R, L, VL> {
        VersionJsonInstallPlanner {
            version: &self.version,
            metadata: &self.metadata,
            asset_index_objects: &self.asset_index_objects,
            remote_resolver: self.remote_resolver.clone(),
            rule_context: VersionJsonRuleContext::current(),
        }
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }

    pub async fn status(&self) -> Result<VersionJsonInstallStatus> {
        let rule_context = VersionJsonRuleContext::current();

        Ok(VersionJsonInstallStatus {
            metadata_persisted: self
                .version
                .try_get_resource(VersionJsonInstanceResource::Metadata)?
                .exists(),
            asset_index_persisted: self
                .version
                .parent
                .try_get_resource(VersionJsonRootResource::AssetIndexes(Some(
                    self.metadata.asset_index.id.clone(),
                )))?
                .exists(),
            version_artifacts_ready: self.version_artifacts_ready(&rule_context)?,
            assets_ready: self.assets_ready()?,
            natives_extracted: self.version.natives_are_extracted().await,
        })
    }

    pub async fn prepare(
        self,
        session: &DownloadSession,
    ) -> Result<PreparedVersionJsonInstance<R, L, VL>> {
        let status = self.status().await?;
        if !status.is_downloaded() {
            let batches = self.planner().download_batches()?;
            for batch in batches {
                session.submit(batch).await?;
            }
            session.success().await?;
        }

        self.finish_preparation().await
    }

    pub async fn submit_install_batches(&self, session: &DownloadSession) -> Result<()> {
        for batch in self.planner().download_batches()? {
            session.submit(batch).await?;
        }
        Ok(())
    }

    pub async fn finish_preparation(self) -> Result<PreparedVersionJsonInstance<R, L, VL>> {
        let status = self.status().await?;
        if !status.natives_extracted {
            self.version.extract_natives().await?;
        }

        self.into_prepared().await
    }

    fn version_artifacts_ready(&self, rule_context: &VersionJsonRuleContext) -> Result<bool> {
        if !self
            .version
            .try_get_resource(VersionJsonInstanceResource::Jar)?
            .exists()
        {
            return Ok(false);
        }

        for library in &self.metadata.libraries {
            if !library.is_allowed(rule_context) {
                continue;
            }

            for artifact in library
                .version_artifacts(rule_context.platform())
                .into_iter()
                .flatten()
            {
                if !self
                    .version
                    .parent
                    .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                        artifact.path.as_str(),
                    ))))?
                    .exists()
                {
                    return Ok(false);
                }
            }
        }

        if let Some(logging) = &self.metadata.logging
            && let Some(client) = &logging.client
            && !self
                .version
                .parent
                .try_get_resource(VersionJsonRootResource::AssetLogConfigs(Some(
                    client.file.id.clone(),
                )))?
                .exists()
        {
            return Ok(false);
        }

        Ok(true)
    }

    fn assets_ready(&self) -> Result<bool> {
        for object in self.asset_index_objects.objects.values() {
            if !self
                .version
                .parent
                .try_get_resource(VersionJsonRootResource::AssetObjects(Some(
                    object.hash.clone(),
                )))?
                .exists()
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl<R, L, VL> PreparedVersionJsonInstance<R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn required_java_major_version(&self) -> usize {
        self.resolved_version.required_java_major_version()
    }
}

impl<'a, R, L, VL> VersionJsonInstallPlanner<'a, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    fn batch_version_artifacts(&self) -> Result<DownloadBatch> {
        let mut tasks = Vec::new();
        tasks.push(
            DownloadTask::new(
                self.remote_resolver
                    .rewrite_upstream(self.metadata.downloads.client.url.as_str())?,
                self.version
                    .try_get_resource(VersionJsonInstanceResource::Jar)?,
            )
            .size(self.metadata.downloads.client.size as u64)
            .sha1(self.metadata.downloads.client.sha1.clone()),
        );

        for library in &self.metadata.libraries {
            tasks.extend(self.plan_library_tasks(library)?);
        }

        if let Some(logging) = &self.metadata.logging
            && let Some(client) = &logging.client
        {
            tasks.push(
                DownloadTask::new(
                    self.remote_resolver
                        .rewrite_upstream(client.file.url.as_str())?,
                    self.version.parent.try_get_resource(
                        VersionJsonRootResource::AssetLogConfigs(Some(client.file.id.clone())),
                    )?,
                )
                .size(client.file.size as u64)
                .sha1(client.file.sha1.clone()),
            );
        }

        Ok(DownloadBatch::new(tasks).group("version"))
    }

    fn plan_library_tasks(&self, library: &PistonMetaLibraries) -> Result<Vec<DownloadTask>> {
        if !library.is_allowed(&self.rule_context) {
            return Ok(Vec::new());
        }

        library
            .version_artifacts(self.rule_context.platform())
            .into_iter()
            .flatten()
            .map(|artifact| self.plan_library_artifact_task(artifact))
            .collect()
    }

    fn plan_library_artifact_task(
        &self,
        artifact: &PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<DownloadTask> {
        let mut task = DownloadTask::new(
            self.remote_resolver
                .rewrite_upstream(artifact.url.as_str())?,
            self.version
                .parent
                .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                    artifact.path.as_str(),
                ))))?,
        );
        if let Some(size) = artifact.size {
            task = task.size(size as u64);
        }
        if let Some(sha1) = &artifact.sha1 {
            task = task.sha1(sha1.clone());
        }
        Ok(task)
    }

    fn batch_assets(&self) -> Result<DownloadBatch> {
        let tasks =
            self.asset_index_objects
                .objects
                .values()
                .map(|object| {
                    Ok(DownloadTask::new(
                        self.remote_resolver.object_url(object.hash.as_str())?,
                        self.version.parent.try_get_resource(
                            VersionJsonRootResource::AssetObjects(Some(object.hash.clone())),
                        )?,
                    )
                    .size(object.size as u64)
                    .sha1(object.hash.clone()))
                })
                .collect::<Result<Vec<DownloadTask>>>()?;

        Ok(DownloadBatch::new(tasks).group("assets"))
    }
}

impl<'a, R, L, VL> VersionJsonInstallPlanner<'a, R, L, VL>
where
    R: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn download_batches(&self) -> Result<Vec<DownloadBatch>> {
        let mut batches = Vec::new();

        let version_batch = self.batch_version_artifacts()?;
        if !version_batch.is_empty() {
            batches.push(version_batch);
        }

        let assets_batch = self.batch_assets()?;
        if !assets_batch.is_empty() {
            batches.push(assets_batch);
        }

        Ok(batches)
    }
}
