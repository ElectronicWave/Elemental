use std::path::Path;

use crate::error::Result;
use elemental_shared::{
    error::Error as SharedError,
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
    let scope = Scope::Directory(instance_root.to_owned());
    let store = match Store::load(
        NoMigrator,
        json_persistor("natives".to_owned(), scope.clone()),
        NATIVES_STATE_VERSION,
    )
    .await
    {
        Ok(store) => store,
        Err(SharedError::Json(_)) | Err(SharedError::StoredVersionNewer { .. }) => {
            remove_stale_natives_state(&scope).await?;
            Store::load(
                NoMigrator,
                json_persistor("natives".to_owned(), scope),
                NATIVES_STATE_VERSION,
            )
            .await?
        }
        Err(error) => return Err(error.into()),
    };

    Ok(store)
}

async fn remove_stale_natives_state(scope: &Scope) -> Result<()> {
    let path = scope
        .get_full_path("natives", Some("json".to_owned()))
        .await?;
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
