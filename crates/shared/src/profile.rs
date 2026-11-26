use crate::{
    error::Result,
    loader::Loader,
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

pub struct ProfileLoader<M, S, C, P>
where
    M: Migrator<S, Profile<C>>,
    S: Versioned,
    P: Persistor<S>,
{
    loader: Loader<M, S, Profile<C>, P>,
}

impl<M, S, C, P> Clone for ProfileLoader<M, S, C, P>
where
    M: Migrator<S, Profile<C>>,
    S: Versioned,
    P: Persistor<S>,
{
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
        }
    }
}

impl<M, S, C, P> ProfileLoader<M, S, C, P>
where
    M: Migrator<S, Profile<C>>,
    S: Versioned,
    C: Default,
    P: Persistor<S>,
{
    pub async fn load(migrator: M, persistor: P, target_version: usize) -> Result<Self> {
        Ok(Self {
            loader: Loader::load(migrator, persistor, target_version).await?,
        })
    }

    pub async fn cloned(&self) -> C
    where
        C: Clone,
    {
        self.loader.get(|profile| profile.config.clone()).await
    }

    pub async fn version(&self) -> usize {
        self.loader.get(|profile| profile.version).await
    }

    pub async fn get<T>(&self, f: impl FnOnce(&C) -> T) -> T {
        self.loader.get(|profile| f(&profile.config)).await
    }

    pub async fn set(&self, f: impl FnOnce(&mut C)) -> Result<()> {
        self.loader.set(|profile| f(&mut profile.config)).await
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
