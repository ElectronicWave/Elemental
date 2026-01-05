use crate::{
    loader::ProfileLoader,
    version::{Migrator, Persistor, VersionControlled},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Profile<C> {
    /// Profile name
    pub name: String,
    /// Profile configuration data
    pub config: C,
    pub version: usize,
}

impl<C: Default> VersionControlled for Profile<C> {
    #[inline(always)]
    fn version(&self) -> usize {
        self.version
    }
}

impl<C: Default> Profile<C> {
    pub async fn load<M: Migrator<Profile<C>>, P: Persistor<Profile<C>>>(
        migrator: M,
        persistor: P,
        version: usize,
    ) -> Result<ProfileLoader<M, C, P>> {
        Ok(ProfileLoader::load(migrator, persistor, version).await?)
    }
}

impl<C: Clone> Clone for Profile<C> {
    fn clone(&self) -> Self {
        Profile {
            name: self.name.clone(),
            config: self.config.clone(),
            version: self.version,
        }
    }
}
