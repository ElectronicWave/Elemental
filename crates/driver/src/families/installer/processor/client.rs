use std::path::PathBuf;

use elemental_core::runtime::distribution::Distribution;
use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::{
        installer::{InstallerArchive, InstallerArtifact},
        version_json::InstanceStorage,
    },
};

use super::{
    context::{CLIENT_PROCESSOR_DATA_KEYS, InstallerProcessorContext},
    processor_applies_to_client,
    runner::run_processor,
};

pub type ProcessorObserver<'a> = &'a (dyn Fn(usize, usize, &str) + Send + Sync);

pub async fn run_installer_client_processors(
    runtime: &Distribution,
    instance: &InstanceStorage,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    on_processor: Option<ProcessorObserver<'_>>,
) -> Result<()> {
    let client_processors = install_profile
        .processors
        .iter()
        .filter(|processor| processor_applies_to_client(processor))
        .collect::<Vec<&ForgeInstallerProcessor>>();
    if client_processors.is_empty() {
        return Ok(());
    }

    let context = build_context(instance, installer_artifact, install_profile, family_name)?;
    if client_outputs_ready(&context).await? {
        return Ok(());
    }

    ensure_client_directories(&context).await?;
    let total = client_processors.len();
    for (index, processor) in client_processors.into_iter().enumerate() {
        if let Some(observer) = on_processor {
            observer(index + 1, total, processor.jar.as_str());
        }
        run_processor(runtime, &context, processor).await?;
    }

    if !client_outputs_ready(&context).await? {
        return Err(Error::ProcessorOutputsMissing {
            family_name: family_name.to_owned(),
        });
    }

    Ok(())
}

pub async fn installer_client_processors_ready(
    instance: &InstanceStorage,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<bool> {
    if !super::installer_has_client_processors(install_profile) {
        return Ok(true);
    }

    let context = build_context(instance, installer_artifact, install_profile, family_name)?;
    client_outputs_ready(&context).await
}

async fn ensure_client_directories(context: &InstallerProcessorContext<'_>) -> Result<()> {
    for path in required_client_output_paths(context).await? {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
    }

    Ok(())
}

async fn client_outputs_ready(context: &InstallerProcessorContext<'_>) -> Result<bool> {
    let paths = required_client_output_paths(context).await?;
    if paths.is_empty() {
        return Ok(false);
    }

    Ok(paths.iter().all(|path| path.exists()))
}

async fn required_client_output_paths(
    context: &InstallerProcessorContext<'_>,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for key in CLIENT_PROCESSOR_DATA_KEYS {
        if let Some(value) = context.resolve_data_value(key).await? {
            paths.push(PathBuf::from(value));
        }
    }

    Ok(paths)
}

fn build_context<'a>(
    instance: &InstanceStorage,
    installer_artifact: &'a InstallerArtifact,
    install_profile: &'a ForgeInstallerProfile,
    family_name: &'a str,
) -> Result<InstallerProcessorContext<'a>> {
    InstallerProcessorContext::new(
        instance,
        installer_artifact,
        install_profile,
        InstallerArchive::new(installer_artifact.path.clone()),
        family_name,
    )
}
