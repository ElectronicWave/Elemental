use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};

use crate::{
    error::{Error, Result},
    families::{
        installer::{InstallerArchive, InstallerArtifact},
        version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    },
};

use super::{
    context::InstallerProcessorContext, processor_applies_to_client, runner::run_processor,
};

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
    if context.client_outputs_ready().await? {
        return Ok(());
    }

    context.ensure_client_directories().await?;
    for processor in client_processors {
        run_processor(runtime, &context, processor).await?;
    }

    if !context.client_outputs_ready().await? {
        return Err(Error::ProcessorOutputsMissing {
            family_name: family_name.to_owned(),
        });
    }

    Ok(())
}

pub async fn installer_client_processors_ready<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    if !super::installer_has_client_processors(install_profile) {
        return Ok(true);
    }

    build_context(instance, installer_artifact, install_profile, family_name)?
        .client_outputs_ready()
        .await
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
