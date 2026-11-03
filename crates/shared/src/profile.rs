use crate::{
    error::Result,
    loader::ProfileLoader,
    migrator::Migrator,
    version::{Persistor, VersionControlled, Versioned},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Profile<C> {
    /// Profile configuration data
    pub config: C,
    pub version: usize,
}

impl<C> Versioned for Profile<C> {
    #[inline(always)]
    fn version(&self) -> usize {
        self.version
    }
}

impl<C> VersionControlled for Profile<C> {
    #[inline(always)]
    fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }
}

impl<C: Default> Profile<C> {
    pub async fn load<M, S, P>(
        migrator: M,
        persistor: P,
        target_version: usize,
    ) -> Result<ProfileLoader<M, S, C, P>>
    where
        S: Versioned,
        M: Migrator<S, Profile<C>>,
        P: Persistor<S>,
    {
        ProfileLoader::load(migrator, persistor, target_version).await
    }
}

impl<C: Clone> Clone for Profile<C> {
    fn clone(&self) -> Self {
        Profile {
            config: self.config.clone(),
            version: self.version,
        }
    }
}
