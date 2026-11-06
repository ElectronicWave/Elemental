use std::path::{Path, PathBuf};

use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};
use tokio::fs::create_dir_all;

use crate::{
    error::{Error, Result},
    families::{
        installer::{InstallerArchive, InstallerArtifact, installer_coordinate_path},
        version_json::{
            VersionJsonInstanceLayout, VersionJsonInstanceResource, VersionJsonRootLayout,
            VersionJsonRootResource,
        },
    },
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

impl<'a> InstallerProcessorContext<'a> {
    pub(super) fn new<L, VL>(
        instance: &Storage<VL, Storage<L>>,
        installer_artifact: &'a InstallerArtifact,
        install_profile: &'a ForgeInstallerProfile,
        archive: InstallerArchive<PathBuf>,
        family_name: &'a str,
    ) -> Result<Self>
    where
        L: VersionJsonRootLayout,
        VL: VersionJsonInstanceLayout,
    {
        Ok(Self {
            installer_artifact,
            install_profile,
            archive,
            root_directory: absolute_path(&instance.parent.path)?,
            libraries_directory: absolute_path(
                &instance
                    .parent
                    .try_get_resource(VersionJsonRootResource::Libraries(None))?,
            )?,
            state_directory: absolute_path(&instance.try_get_resource(
                VersionJsonInstanceResource::Elemental(Some(
                    PathBuf::from(family_name).join("installer"),
                )),
            )?)?,
            minecraft_jar_path: absolute_path(
                &instance.try_get_resource(VersionJsonInstanceResource::Jar)?,
            )?,
            family_name,
        })
    }

    pub(super) async fn ensure_client_directories(&self) -> Result<()> {
        for path in self.required_client_output_paths()? {
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await?;
            }
        }

        Ok(())
    }

    pub(super) fn client_outputs_ready(&self) -> Result<bool> {
        let paths = self.required_client_output_paths()?;
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
            .join(installer_coordinate_path(processor.jar.as_str())?))
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
                    .join(installer_coordinate_path(coordinate)?),
            );
        }

        Ok(entries)
    }

    fn required_client_output_paths(&self) -> Result<Vec<PathBuf>> {
        CLIENT_PROCESSOR_DATA_KEYS
            .iter()
            .map(|key| self.resolve_data_value(key))
            .collect::<Result<Vec<Option<String>>>>()
            .map(|values| values.into_iter().flatten().map(PathBuf::from).collect())
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let current_directory = std::env::current_dir().map_err(|source| Error::CurrentDirectory {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(current_directory.join(path))
}
