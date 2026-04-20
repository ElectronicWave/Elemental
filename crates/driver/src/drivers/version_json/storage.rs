use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use elemental_core::{jar::JarFile, storage::Storage};
use tokio::fs::create_dir_all;

use super::layout::{VersionJsonInstanceLayout, VersionJsonRootLayout};
use crate::{
    driver::Driver,
    drivers::version_json::{
        PistonMetaAssetIndexObjects, PistonMetaData,
        extensions::PistonMetaLibrariesExt,
        rules::VersionJsonRuleContext,
        state::{NativesState, natives_state_store},
    },
    inspect::{InstalledInstance, inspect_instance},
};

#[async_trait(?Send)]
pub trait VersionJsonGameStorageExt {
    type Layout: VersionJsonRootLayout;

    fn instances_root_path(&self) -> Result<PathBuf>;
    fn instance_root_path(&self, name: impl AsRef<str>) -> Result<PathBuf>;
    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send;
    fn instance_exists(&self, name: impl AsRef<str>) -> Result<bool>;
    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout;
    fn instance_names(&self) -> Result<Vec<String>>;
    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone;
    fn asset_index_path(&self, id: impl AsRef<str>) -> Result<PathBuf>;
    fn asset_object_path(&self, hash: impl AsRef<str>) -> Result<PathBuf>;
    fn library_path(&self, path: impl AsRef<Path>) -> Result<PathBuf>;
    fn logging_config_path(&self, file_id: impl AsRef<str>) -> Result<PathBuf>;
    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()>;
    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects>;
}

#[async_trait(?Send)]
impl<L> VersionJsonGameStorageExt for Storage<L>
where
    L: VersionJsonRootLayout,
{
    type Layout = L;

    fn instances_root_path(&self) -> Result<PathBuf> {
        Ok(self.layout.instances_root_path(&self.path))
    }

    fn instance_root_path(&self, name: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.instances_root_path()?.join(name.as_ref()))
    }

    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send,
    {
        let instances_root = self.instances_root_path()?;
        create_dir_all(&instances_root).await?;

        let instance_root = instances_root.join(&name);
        create_dir_all(&instance_root).await?;

        Ok(Storage::with_parent(
            instance_root,
            self.clone(),
            version_layout,
        ))
    }

    fn instance_exists(&self, name: impl AsRef<str>) -> Result<bool> {
        let name = name.as_ref();
        let instance_root = self.instance_root_path(name)?;

        Ok(instance_root.join(format!("{name}.jar")).exists()
            && instance_root.join(format!("{name}.json")).exists())
    }

    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout,
    {
        let name = name.into();
        if !self.instance_exists(&name)? {
            return Err(anyhow!("can't find a valid instance named '{name}'"));
        }

        Ok(Storage::with_parent(
            self.instance_root_path(&name)?,
            self.clone(),
            version_layout,
        ))
    }

    fn instance_names(&self) -> Result<Vec<String>> {
        let instances_root = self.instances_root_path()?;
        if !instances_root.exists() {
            return Ok(Vec::new());
        }

        let mut instances = Vec::new();
        for entry in instances_root.read_dir()? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if self.instance_exists(&name)? {
                instances.push(name);
            }
        }

        Ok(instances)
    }

    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone,
    {
        self.instance_names()?
            .into_iter()
            .map(|name| self.instance(name, version_layout.clone()))
            .collect()
    }

    fn asset_index_path(&self, id: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.layout.asset_index_path(&self.path, id.as_ref()))
    }

    fn asset_object_path(&self, hash: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self.layout.asset_object_path(&self.path, hash.as_ref()))
    }

    fn library_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        Ok(self.layout.library_path(&self.path, path.as_ref()))
    }

    fn logging_config_path(&self, file_id: impl AsRef<str>) -> Result<PathBuf> {
        Ok(self
            .layout
            .logging_config_path(&self.path, file_id.as_ref()))
    }

    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()> {
        let path = self.asset_index_path(id)?;
        let parent = path
            .parent()
            .context("asset index path has no parent directory")?;
        create_dir_all(parent).await?;
        tokio::fs::write(path, serde_json::to_vec(objects)?).await?;
        Ok(())
    }

    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects> {
        Ok(serde_json::from_reader(File::open(
            self.asset_index_path(id)?,
        )?)?)
    }
}

#[async_trait(?Send)]
pub trait VersionJsonVersionStorageExt {
    type GameLayout: VersionJsonRootLayout;
    type VersionLayout: VersionJsonInstanceLayout;

    fn metadata_path(&self) -> Result<PathBuf>;
    fn metadata(&self) -> Result<PistonMetaData>;
    fn jar_path(&self) -> Result<PathBuf>;
    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()>;
    fn platform_natives_path(&self) -> PathBuf;
    async fn natives_are_extracted(&self) -> bool;
    async fn ensure_platform_natives_path(&self) -> Result<PathBuf>;
    async fn extract_natives(&self) -> Result<()>;
}

#[async_trait(?Send)]
impl<L, VL> VersionJsonVersionStorageExt for Storage<VL, Storage<L>>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    type GameLayout = L;
    type VersionLayout = VL;

    fn metadata_path(&self) -> Result<PathBuf> {
        let name = self.name().context("get version name failed")?;
        Ok(self.layout.metadata_path(&self.path, &name))
    }

    fn metadata(&self) -> Result<PistonMetaData> {
        Ok(serde_json::from_reader(File::open(self.metadata_path()?)?)?)
    }

    fn jar_path(&self) -> Result<PathBuf> {
        let name = self.name().context("get version name failed")?;
        Ok(self.layout.jar_path(&self.path, &name))
    }

    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()> {
        self.ensure_root().await?;
        tokio::fs::write(self.metadata_path()?, serde_json::to_vec(metadata)?).await?;
        Ok(())
    }

    fn platform_natives_path(&self) -> PathBuf {
        self.layout.platform_natives_path(&self.path)
    }

    async fn natives_are_extracted(&self) -> bool {
        let metadata = match self.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };
        let rule_context = VersionJsonRuleContext::current();
        let natives_root = self.platform_natives_path();
        let store = match natives_state_store(&self.path).await {
            Ok(store) => store,
            Err(_) => return false,
        };
        let state = store.cloned().await;
        if state.value.path.is_empty() {
            return false;
        }

        let expected_path = natives_root.to_string_lossy().to_string();
        if state.value.path != expected_path {
            return false;
        }

        let expected_artifacts = collect_native_artifact_paths(&metadata, &rule_context);
        if state.value.native_artifacts != expected_artifacts {
            return false;
        }

        if state.value.extracted_files.is_empty() {
            return false;
        }

        state
            .value
            .extracted_files
            .iter()
            .all(|file| natives_root.join(file).exists())
    }

    async fn ensure_platform_natives_path(&self) -> Result<PathBuf> {
        let path = self.platform_natives_path();
        create_dir_all(&path).await?;
        Ok(path)
    }

    async fn extract_natives(&self) -> Result<()> {
        let metadata = self.metadata()?;
        let destination = self.platform_natives_path();
        let rule_context = VersionJsonRuleContext::current();
        std::fs::create_dir_all(&destination)?;

        tokio::task::block_in_place(|| -> Result<()> {
            for library in &metadata.libraries {
                if !library.is_allowed(&rule_context) {
                    continue;
                }

                if let Some(artifact) = library.classifiers_native_artifact(rule_context.platform())
                {
                    let source = self.parent.library_path(artifact.path.as_str())?;
                    JarFile::new(source).extract_blocking(&destination)?;
                }

                if let Some(artifact) = library.native_artifact(rule_context.platform()) {
                    let source = self.parent.library_path(artifact.path.as_str())?;
                    JarFile::new(source).extract_blocking(&destination)?;
                }
            }

            flatten_native_binaries(&destination)
        })?;
        let extracted_files = collect_root_native_binaries(&destination)?
            .into_iter()
            .filter_map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .collect::<Vec<String>>();
        let checked_at_unix_ms = current_unix_ms()?;
        let store = natives_state_store(&self.path).await?;
        store
            .set(|state| {
                state.value = NativesState {
                    path: destination.to_string_lossy().to_string(),
                    native_artifacts: collect_native_artifact_paths(&metadata, &rule_context),
                    extracted_files,
                    checked_at_unix_ms,
                };
            })
            .await?;

        Ok(())
    }
}

pub async fn inspect_instances<L, VL>(
    storage: &Storage<L>,
    version_layout: VL,
    drivers: &[&dyn Driver<L, VL>],
) -> Result<Vec<InstalledInstance<L, VL>>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let mut instances = Vec::new();
    for instance in storage.instances(version_layout)? {
        if let Some(installed) = inspect_instance(instance, drivers).await? {
            instances.push(installed);
        }
    }

    Ok(instances)
}

fn current_unix_ms() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before unix epoch")?;
    Ok(duration.as_millis() as u64)
}

fn flatten_native_binaries(root: &Path) -> Result<()> {
    for path in collect_native_binaries(root)? {
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let target = root.join(file_name);
        if target == path {
            continue;
        }

        std::fs::copy(&path, &target).with_context(|| {
            format!(
                "copy native binary '{}' to '{}' failed",
                path.display(),
                target.display()
            )
        })?;
    }

    Ok(())
}

fn collect_root_native_binaries(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file() && is_native_binary(&path) {
            files.push(path);
        }
    }
    Ok(files)
}

fn collect_native_artifact_paths(
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
) -> Vec<String> {
    let mut paths = Vec::new();

    for library in &metadata.libraries {
        if !library.is_allowed(rule_context) {
            continue;
        }

        if let Some(artifact) = library.classifiers_native_artifact(rule_context.platform()) {
            paths.push(artifact.path.clone());
        }

        if let Some(artifact) = library.native_artifact(rule_context.platform()) {
            paths.push(artifact.path.clone());
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

fn collect_native_binaries(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_native_binaries_into(root, &mut files)?;
    Ok(files)
}

fn collect_native_binaries_into(current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_native_binaries_into(&path, files)?;
            continue;
        }

        if is_native_binary(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_native_binary(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "dll" | "so" | "dylib" | "jnilib"
            )
        })
}
