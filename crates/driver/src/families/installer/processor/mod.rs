mod context;
mod runner;
mod variables;

use std::{collections::HashSet, path::PathBuf};

use elemental_core::{
    runtime::distribution::Distribution,
    storage::{Storage, layout::Layoutable},
};
use elemental_infra::downloader::{DownloadBatch, DownloadSession, DownloadTask};
use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};

use crate::{
    error::{Error, Result},
    families::{
        installer::{InstallerArchive, InstallerArtifact, libraries::installer_library_artifacts},
        version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout, VersionJsonRootResource},
    },
};

use context::InstallerProcessorContext;
use runner::run_processor;

pub async fn submit_installer_profile_libraries<L, VL, F>(
    session: &DownloadSession,
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    artifact_url: F,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
    F: Fn(&str, &str) -> Result<String>,
{
    let tasks = install_profile_library_tasks(instance, install_profile, &artifact_url)?;

    if tasks.is_empty() {
        return Ok(());
    }

    session
        .submit(DownloadBatch::new(tasks).group("profile-libraries"))
        .await
        .map_err(|source| Error::DownloadInstallerProfileLibraries {
            family_name: family_name.to_owned(),
            source,
        })
}

pub async fn run_installer_client_processors<L, VL>(
    runtime: &Distribution,
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let client_processors = install_profile
        .processors
        .iter()
        .filter(|processor| processor_applies_to_client(processor))
        .collect::<Vec<&ForgeInstallerProcessor>>();
    if client_processors.is_empty() {
        return Ok(());
    }

    let context = build_context(instance, installer_artifact, install_profile, family_name)?;
    if context.client_outputs_ready()? {
        return Ok(());
    }

    context.ensure_client_directories().await?;
    for processor in client_processors {
        run_processor(runtime, &context, processor).await?;
    }

    if !context.client_outputs_ready()? {
        return Err(Error::ProcessorOutputsMissing {
            family_name: family_name.to_owned(),
        });
    }

    Ok(())
}

pub fn installer_has_client_processors(install_profile: &ForgeInstallerProfile) -> bool {
    install_profile
        .processors
        .iter()
        .any(processor_applies_to_client)
}

pub fn installer_client_processors_ready<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    if !installer_has_client_processors(install_profile) {
        return Ok(true);
    }

    build_context(instance, installer_artifact, install_profile, family_name)?
        .client_outputs_ready()
}

fn processor_applies_to_client(processor: &ForgeInstallerProcessor) -> bool {
    processor.sides.is_empty() || processor.sides.iter().any(|side| side == "client")
}

fn build_context<'a, L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &'a InstallerArtifact,
    install_profile: &'a ForgeInstallerProfile,
    family_name: &'a str,
) -> Result<InstallerProcessorContext<'a>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    InstallerProcessorContext::new(
        instance,
        installer_artifact,
        install_profile,
        InstallerArchive::new(installer_artifact.path.clone()),
        family_name,
    )
}

fn install_profile_library_tasks<L, VL, F>(
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
    artifact_url: &F,
) -> Result<Vec<DownloadTask>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
    F: Fn(&str, &str) -> Result<String>,
{
    let mut seen = HashSet::new();
    let mut tasks = Vec::new();

    for artifact in install_profile
        .libraries
        .iter()
        .flat_map(installer_library_artifacts)
    {
        let path = instance
            .parent
            .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                artifact.path.as_str(),
            ))))?;

        if !seen.insert(path.clone()) {
            continue;
        }

        let mut task = DownloadTask::new(
            artifact_url(artifact.url.as_str(), artifact.path.as_str())?,
            path,
        );
        if let Some(size) = artifact.size {
            task = task.size(size as u64);
        }
        if let Some(sha1) = &artifact.sha1 {
            task = task.sha1(sha1.clone());
        }
        tasks.push(task);
    }

    Ok(tasks)
}
