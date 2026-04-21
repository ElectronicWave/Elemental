use anyhow::Result;
use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_infra::downloader::core::ElementalDownloader;
use elemental_schema::forge::ForgeInstallerProfile;

use crate::{
    drivers::forge::prepared::ForgeRemoteResolver,
    families::{
        installer::{
            InstallerArtifact, ensure_installer_profile_libraries_downloaded,
            installer_client_processors_ready, run_installer_client_processors,
        },
        version_json::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    },
};

pub async fn ensure_profile_libraries_downloaded<L, VL>(
    downloader: &ElementalDownloader,
    remote_resolver: &ForgeRemoteResolver,
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    ensure_installer_profile_libraries_downloaded(
        downloader,
        instance,
        install_profile,
        "forge",
        |raw_url, artifact_path| remote_resolver.forge_artifact_url(raw_url, artifact_path),
    )
    .await
}

pub async fn run_client_processors<L, VL>(
    runtime: &Distribution,
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    run_installer_client_processors(
        runtime,
        instance,
        installer_artifact,
        install_profile,
        "forge",
    )
    .await
}

pub fn client_processors_ready<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    installer_client_processors_ready(instance, installer_artifact, install_profile, "forge")
}
