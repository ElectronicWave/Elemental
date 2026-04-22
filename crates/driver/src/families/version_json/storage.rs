use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::jar::JarFile;
use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};
use tokio::fs::create_dir_all;

use super::{
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    resource::{VersionJsonInstanceResource, VersionJsonRootResource},
};
use crate::{
    driver::Driver,
    families::version_json::{
        extensions::PistonMetaLibrariesExt,
        rules::VersionJsonRuleContext,
        state::{NativesState, natives_state_store},
    },
    inspect::InstalledInstance,
};

#[async_trait(?Send)]
pub trait VersionJsonGameStorageExt {
    type Layout: VersionJsonRootLayout;

    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send;
    fn instance_exists<VL>(&self, name: impl AsRef<str>, version_layout: &VL) -> Result<bool>
    where
        VL: VersionJsonInstanceLayout;
    fn instance<VL>(
        &self,
        name: impl Into<String>,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout;
    fn instance_names<VL>(&self, version_layout: &VL) -> Result<Vec<String>>
    where
        VL: VersionJsonInstanceLayout;
    fn instances<VL>(&self, version_layout: VL) -> Result<Vec<Storage<VL, Storage<Self::Layout>>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Clone;
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

    async fn ensure_instance<VL>(
        &self,
        name: String,
        version_layout: VL,
    ) -> Result<Storage<VL, Storage<Self::Layout>>>
    where
        Self::Layout: Clone,
        VL: VersionJsonInstanceLayout + Send,
    {
        let instance_root = self.try_get_resource(VersionJsonRootResource::Versions(Some(name)))?;
        create_dir_all(&instance_root).await?;

        Ok(Storage::with_parent(
            instance_root,
            self.clone(),
            version_layout,
        ))
    }

    fn instance_exists<VL>(&self, name: impl AsRef<str>, version_layout: &VL) -> Result<bool>
    where
        VL: VersionJsonInstanceLayout,
    {
        let name = name.as_ref();
        let instance_root =
            self.try_get_resource(VersionJsonRootResource::Versions(Some(name.to_owned())))?;
        let metadata_path = version_layout
            .try_get_resource(&instance_root, VersionJsonInstanceResource::Metadata)?;
        let jar_path =
            version_layout.try_get_resource(&instance_root, VersionJsonInstanceResource::Jar)?;

        Ok(metadata_path.exists() && jar_path.exists())
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
        if !self.instance_exists(&name, &version_layout)? {
            return Err(anyhow!("can't find a valid instance named '{name}'"));
        }
        let instance_root = self.try_get_resource(VersionJsonRootResource::Versions(Some(name)))?;

        Ok(Storage::with_parent(
            instance_root,
            self.clone(),
            version_layout,
        ))
    }

    fn instance_names<VL>(&self, version_layout: &VL) -> Result<Vec<String>>
    where
        VL: VersionJsonInstanceLayout,
    {
        let instances_root = self.try_get_resource(VersionJsonRootResource::Versions(None))?;
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
            if self.instance_exists(&name, version_layout)? {
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
        self.instance_names(&version_layout)?
            .into_iter()
            .map(|name| self.instance(name, version_layout.clone()))
            .collect()
    }

    async fn write_asset_index(
        &self,
        id: String,
        objects: &PistonMetaAssetIndexObjects,
    ) -> Result<()> {
        let path = self.try_get_resource(VersionJsonRootResource::AssetIndexes(Some(id)))?;
        let parent = path
            .parent()
            .context("asset index path has no parent directory")?;
        create_dir_all(parent).await?;
        tokio::fs::write(path, serde_json::to_vec(objects)?).await?;
        Ok(())
    }

    fn asset_index_objects(&self, id: impl AsRef<str>) -> Result<PistonMetaAssetIndexObjects> {
        let path = self.try_get_resource(VersionJsonRootResource::AssetIndexes(Some(
            id.as_ref().to_owned(),
        )))?;
        Ok(serde_json::from_reader(File::open(path)?)?)
    }
}

#[async_trait(?Send)]
pub trait VersionJsonVersionStorageExt {
    type GameLayout: VersionJsonRootLayout;
    type VersionLayout: VersionJsonInstanceLayout;

    fn metadata(&self) -> Result<PistonMetaData>;
    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()>;
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

    fn metadata(&self) -> Result<PistonMetaData> {
        let path = self.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        Ok(serde_json::from_reader(File::open(path)?)?)
    }

    async fn write_metadata(&self, metadata: &PistonMetaData) -> Result<()> {
        self.ensure_root().await?;
        let path = self.try_get_resource(VersionJsonInstanceResource::Metadata)?;
        tokio::fs::write(path, serde_json::to_vec(metadata)?).await?;
        Ok(())
    }

    async fn natives_are_extracted(&self) -> bool {
        let metadata = match self.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };
        let rule_context = VersionJsonRuleContext::current();
        let natives_root = match self.try_get_resource(VersionJsonInstanceResource::Natives) {
            Ok(path) => path,
            Err(_) => return false,
        };
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

        if expected_artifacts.is_empty() {
            return true;
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
        let path = self.try_get_resource(VersionJsonInstanceResource::Natives)?;
        create_dir_all(&path).await?;
        Ok(path)
    }

    async fn extract_natives(&self) -> Result<()> {
        let metadata = self.metadata()?;
        let destination = self.try_get_resource(VersionJsonInstanceResource::Natives)?;
        let rule_context = VersionJsonRuleContext::current();
        std::fs::create_dir_all(&destination)?;

        tokio::task::block_in_place(|| -> Result<()> {
            let natives_directory = NativesDirectory::new(&destination);

            for library in &metadata.libraries {
                if !library.is_allowed(&rule_context) {
                    continue;
                }

                if let Some(artifact) = library.classifiers_native_artifact(rule_context.platform())
                {
                    let source =
                        self.parent
                            .try_get_resource(VersionJsonRootResource::Libraries(Some(
                                PathBuf::from(artifact.path.as_str()),
                            )))?;
                    JarFile::new(source).extract_blocking(&destination)?;
                }

                if let Some(artifact) = library.native_artifact(rule_context.platform()) {
                    let source =
                        self.parent
                            .try_get_resource(VersionJsonRootResource::Libraries(Some(
                                PathBuf::from(artifact.path.as_str()),
                            )))?;
                    JarFile::new(source).extract_blocking(&destination)?;
                }
            }

            natives_directory.flatten_binaries()
        })?;
        let extracted_files = NativesDirectory::new(&destination)
            .root_binaries()?
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
    InstalledInstance::detect_all(storage, version_layout, drivers).await
}

fn current_unix_ms() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before unix epoch")?;
    Ok(duration.as_millis() as u64)
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

#[derive(Debug, Clone, Copy)]
struct NativesDirectory<'a> {
    root: &'a Path,
}

impl<'a> NativesDirectory<'a> {
    fn new(root: &'a Path) -> Self {
        Self { root }
    }

    fn flatten_binaries(self) -> Result<()> {
        for path in self.binaries()? {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let target = self.root.join(file_name);
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

    fn root_binaries(self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in std::fs::read_dir(self.root)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_file() && Self::is_binary(&path) {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn binaries(self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.collect_binaries_into(self.root, &mut files)?;
        Ok(files)
    }

    fn collect_binaries_into(self, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                self.collect_binaries_into(&path, files)?;
                continue;
            }

            if Self::is_binary(&path) {
                files.push(path);
            }
        }

        Ok(())
    }

    fn is_binary(path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "dll" | "so" | "dylib" | "jnilib"
                )
            })
    }
}
