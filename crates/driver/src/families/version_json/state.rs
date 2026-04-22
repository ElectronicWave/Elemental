use std::{
    io::ErrorKind,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::Result;
use elemental_core::storage::layout::Layout;
use elemental_shared::{
    migrator::NoMigrator,
    store::{Store, StoreLoader},
    version::{Persistor, VersionControlled},
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, read_to_string, write};

use crate::families::version_json::{BaseInstanceLayout, VersionJsonInstanceResource};

const NATIVES_STATE_VERSION: usize = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NativesState {
    pub path: String,
    pub native_artifacts: Vec<String>,
    pub extracted_files: Vec<String>,
    pub checked_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
pub struct JsonPathPersistor<V> {
    path: PathBuf,
    _marker: PhantomData<V>,
}

impl<V> JsonPathPersistor<V> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            _marker: PhantomData,
        }
    }
}

impl<V> Persistor<V> for JsonPathPersistor<V>
where
    V: VersionControlled + Serialize + DeserializeOwned,
{
    async fn load(&self) -> Result<Option<V>> {
        match read_to_string(&self.path).await {
            Ok(data) => Ok(Some(serde_json::from_str(&data)?)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    async fn save(&self, value: &V) -> Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.exists()
        {
            create_dir_all(parent).await?;
        }

        write(&self.path, serde_json::to_string(value)?).await?;
        Ok(())
    }
}

pub type NativeStateStore =
    StoreLoader<NoMigrator, NativesState, JsonPathPersistor<Store<NativesState>>>;

pub fn natives_state_path(instance_root: &Path) -> Result<PathBuf> {
    BaseInstanceLayout.try_get_extended_resource(
        instance_root,
        VersionJsonInstanceResource::Elemental(Some(PathBuf::from("natives.json"))),
    )
}

pub async fn natives_state_store(instance_root: &Path) -> Result<NativeStateStore> {
    let persistor = JsonPathPersistor::new(natives_state_path(instance_root)?);
    let store = Store::load(NoMigrator, persistor, NATIVES_STATE_VERSION).await?;

    if store.get(|state| state.version).await != NATIVES_STATE_VERSION {
        store
            .set(|state| {
                state.version = NATIVES_STATE_VERSION;
            })
            .await?;
    }

    Ok(store)
}
