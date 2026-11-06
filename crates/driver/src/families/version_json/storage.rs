use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use elemental_core::storage::{Storage, layout::Layoutable};
use elemental_infra::jar::JarFile;
use elemental_schema::mojang::piston::{
    PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaLibrariesDownloadsArtifact,
};
use tokio::fs::create_dir_all;

use super::{
    layout::{VersionJsonInstanceLayout, VersionJsonRootLayout},
    resource::{VersionJsonInstanceResource, VersionJsonRootResource},
};
use crate::error::{Error, Result};
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
        let instance_root = instance_root(self, name)?;
        create_dir_all(&instance_root).await?;

        Ok(open_instance(self, instance_root, version_layout))
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
            return Err(Error::InstanceNotFound { name });
        }
        let instance_root = instance_root(self, name)?;

        Ok(open_instance(self, instance_root, version_layout))
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
        let parent = path.parent().ok_or_else(|| Error::MissingParentDirectory {
            context: "asset index path has no parent directory".to_owned(),
            path: path.clone(),
        })?;
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
        let Some(expected) = expected_natives(self) else {
            return false;
        };
        let Ok(store) = natives_state_store(&self.path).await else {
            return false;
        };
        let state = store.cloned().await;

        natives_state_matches(&state.value, &expected)
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

        extract_native_libraries(self, &metadata, &destination, &rule_context)?;
        record_natives_state(self, &metadata, &destination, &rule_context).await
    }
}

fn instance_root<L>(storage: &Storage<L>, name: String) -> Result<PathBuf>
where
    L: VersionJsonRootLayout,
{
    Ok(storage.try_get_resource(VersionJsonRootResource::Versions(Some(name)))?)
}

fn open_instance<L, VL>(
    storage: &Storage<L>,
    instance_root: PathBuf,
    version_layout: VL,
) -> Storage<VL, Storage<L>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout,
{
    Storage::with_parent(instance_root, storage.clone(), version_layout)
}

fn expected_natives<L, VL>(storage: &Storage<VL, Storage<L>>) -> Option<NativesExpectation>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let metadata = storage.metadata().ok()?;
    let rule_context = VersionJsonRuleContext::current();
    let destination = storage
        .try_get_resource(VersionJsonInstanceResource::Natives)
        .ok()?;

    Some(NativesExpectation {
        destination,
        native_artifacts: collect_native_artifact_paths(&metadata, &rule_context),
    })
}

fn extract_native_libraries<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    destination: &Path,
    rule_context: &VersionJsonRuleContext,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    tokio::task::block_in_place(|| {
        for artifact in native_source_artifacts(metadata, rule_context) {
            let source = storage
                .parent
                .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                    artifact.path.as_str(),
                ))))?;
            JarFile::new(source).extract_blocking(destination)?;
        }

        NativesDirectory::new(destination).flatten_binaries()
    })
}

async fn record_natives_state<L, VL>(
    storage: &Storage<VL, Storage<L>>,
    metadata: &PistonMetaData,
    destination: &Path,
    rule_context: &VersionJsonRuleContext,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let extracted_files = NativesDirectory::new(destination)
        .root_binaries()?
        .into_iter()
        .filter_map(binary_file_name)
        .collect();
    let checked_at_unix_ms = current_unix_ms()?;
    let store = natives_state_store(&storage.path).await?;
    store
        .set(|state| {
            state.value = NativesState {
                path: destination.to_string_lossy().to_string(),
                native_artifacts: collect_native_artifact_paths(metadata, rule_context),
                extracted_files,
                checked_at_unix_ms,
            };
        })
        .await?;

    Ok(())
}

struct NativesExpectation {
    destination: PathBuf,
    native_artifacts: Vec<String>,
}

fn natives_state_matches(state: &NativesState, expected: &NativesExpectation) -> bool {
    if state.path.is_empty() {
        return false;
    }
    if state.path != expected.destination.to_string_lossy() {
        return false;
    }
    if state.native_artifacts != expected.native_artifacts {
        return false;
    }
    if expected.native_artifacts.is_empty() {
        return true;
    }
    if state.extracted_files.is_empty() {
        return false;
    }

    state
        .extracted_files
        .iter()
        .all(|file| expected.destination.join(file).exists())
}

fn binary_file_name(path: PathBuf) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
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
    let duration = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(duration.as_millis() as u64)
}

fn collect_native_artifact_paths(
    metadata: &PistonMetaData,
    rule_context: &VersionJsonRuleContext,
) -> Vec<String> {
    let mut paths = native_source_artifacts(metadata, rule_context)
        .map(|artifact| artifact.path.clone())
        .collect::<Vec<String>>();

    paths.sort();
    paths.dedup();
    paths
}

fn native_source_artifacts<'a>(
    metadata: &'a PistonMetaData,
    rule_context: &'a VersionJsonRuleContext,
) -> impl Iterator<Item = &'a PistonMetaLibrariesDownloadsArtifact> + 'a {
    metadata
        .libraries
        .iter()
        .filter(|library| library.is_allowed(rule_context))
        .flat_map(|library| library.native_source_artifacts(rule_context.platform()))
        .flatten()
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

            std::fs::copy(&path, &target).map_err(|source| Error::CopyNativeBinary {
                source_path: path.clone(),
                target_path: target.clone(),
                source,
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
