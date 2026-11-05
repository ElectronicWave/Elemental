use std::path::Path;

use crate::error::Result;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::{JsonFilePersistor, json_persistor},
    scope::Scope,
    store::{Store, StoreLoader},
};
use serde::{Deserialize, Serialize};

const NATIVES_STATE_VERSION: usize = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NativesState {
    pub path: String,
    pub native_artifacts: Vec<String>,
    pub extracted_files: Vec<String>,
    pub checked_at_unix_ms: u64,
}

pub type NativeStateStore = StoreLoader<
    NoMigrator,
    Store<NativesState>,
    NativesState,
    JsonFilePersistor<Store<NativesState>>,
>;

pub async fn natives_state_store(instance_root: &Path) -> Result<NativeStateStore> {
    let store = Store::load(
        NoMigrator,
        json_persistor(
            "natives".to_owned(),
            Scope::Directory(instance_root.to_owned()),
        ),
        NATIVES_STATE_VERSION,
    )
    .await?;

    Ok(store)
}
