use std::path::PathBuf;

use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};
use tokio::fs::create_dir_all;

use crate::{
    error::Result,
    families::installer::{InstallerArchive, InstallerArtifact, installer_coordinate_path},
};

pub(super) const CLIENT_PROCESSOR_DATA_KEYS: &[&str] = &[
    "MAPPINGS",
    "MOJMAPS",
    "MERGED_MAPPINGS",
    "MC_SLIM",
    "MC_EXTRA",
    "MC_SRG",
    "PATCHED",
];

pub(super) struct InstallerProcessorContext<'a> {
    pub(super) installer_artifact: &'a InstallerArtifact,
    pub(super) install_profile: &'a ForgeInstallerProfile,
    pub(super) archive: InstallerArchive<PathBuf>,
    pub(super) root_directory: PathBuf,
    pub(super) libraries_directory: PathBuf,
    pub(super) state_directory: PathBuf,
    pub(super) minecraft_jar_path: PathBuf,
    pub(super) family_name: &'a str,
}

mod create;

impl<'a> InstallerProcessorContext<'a> {
    pub(super) async fn ensure_client_directories(&self) -> Result<()> {
        for path in self.required_client_output_paths().await? {
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await?;
            }
        }

        Ok(())
    }

    pub(super) async fn client_outputs_ready(&self) -> Result<bool> {
        let paths = self.required_client_output_paths().await?;
        if paths.is_empty() {
            return Ok(false);
        }

        Ok(paths.iter().all(|path| path.exists()))
    }

    pub(super) fn processor_jar_path(
        &self,
        processor: &ForgeInstallerProcessor,
    ) -> Result<PathBuf> {
        Ok(self
            .libraries_directory
            .join(installer_coordinate_path(processor.jar.as_str())?.as_path()))
    }

    pub(super) fn processor_classpath(
        &self,
        processor: &ForgeInstallerProcessor,
    ) -> Result<Vec<PathBuf>> {
        let mut entries = Vec::with_capacity(1 + processor.classpath.len());
        entries.push(self.processor_jar_path(processor)?);

        for coordinate in &processor.classpath {
            entries.push(
                self.libraries_directory
                    .join(installer_coordinate_path(coordinate)?.as_path()),
            );
        }

        Ok(entries)
    }

    async fn required_client_output_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for key in CLIENT_PROCESSOR_DATA_KEYS {
            if let Some(value) = self.resolve_data_value(key).await? {
                paths.push(PathBuf::from(value));
            }
        }

        Ok(paths)
    }
}
