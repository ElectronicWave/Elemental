use elemental_infra::{SafeRelativePath, downloader::DownloadTask};
use elemental_schema::mojang::piston::{PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact};

use crate::{
    error::Result,
    families::version_json::{PistonMetaLibrariesExt, VersionJsonRootResource},
};

use super::VersionJsonInstallPlanner;

impl VersionJsonInstallPlanner<'_> {
    pub(super) fn plan_library_tasks(
        &self,
        library: &PistonMetaLibraries,
    ) -> Result<Vec<DownloadTask>> {
        if !library.is_allowed(&self.rule_context) {
            return Ok(Vec::new());
        }
        let artifacts = library.version_artifacts(self.rule_context.platform())?;
        artifacts
            .into_iter()
            .map(|artifact| self.plan_library_artifact_task(artifact))
            .collect()
    }

    fn plan_library_artifact_task(
        &self,
        artifact: PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<DownloadTask> {
        let mut task = DownloadTask::new(
            self.remote_resolver
                .rewrite_upstream(artifact.url.as_str())?,
            self.version
                .try_get_parent_resource(VersionJsonRootResource::Libraries(Some(
                    SafeRelativePath::parse(artifact.path.as_str())?,
                )))?,
        );
        if let Some(size) = artifact.size {
            task = task.size(size as u64);
        }
        if let Some(sha1) = &artifact.sha1 {
            task = task.sha1(sha1.clone());
        }
        Ok(task)
    }
}
