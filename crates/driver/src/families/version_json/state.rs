use std::path::Path;

use anyhow::Result;
use elemental_shared::{
    migrator::NoMigrator,
    persistor::json_persistor,
    scope::Scope,
    store::{Store, StoreLoader},
    version::Persistor,
};
use serde::{Deserialize, Serialize};

const NATIVES_STATE_ID: &str = "natives";
const NATIVES_STATE_VERSION: usize = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NativesState {
    pub path: String,
    pub native_artifacts: Vec<String>,
    pub extracted_files: Vec<String>,
    pub checked_at_unix_ms: u64,
}

pub type NativeStateStore<P> = StoreLoader<NoMigrator, NativesState, P>;

pub async fn natives_state_store(
    instance_root: &Path,
) -> Result<NativeStateStore<impl Persistor<Store<NativesState>>>> {
    let persistor = json_persistor::<Store<NativesState>>(
        NATIVES_STATE_ID.to_owned(),
        Scope::Custom(instance_root.to_path_buf()),
    );
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
