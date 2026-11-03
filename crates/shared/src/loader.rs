use std::{marker::PhantomData, sync::Arc};

use crate::{
    error::{Error, Result},
    migrator::Migrator,
    profile::Profile,
    version::{Persistor, VersionControlled, Versioned},
};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Loader<M, S, V, P>
where
    M: Migrator<S, V>,
    S: Versioned,
    V: VersionControlled,
    P: Persistor<S>,
{
    pub inner: Arc<RwLock<V>>,
    pub migrator: M,
    pub persistor: P,
    pub target_version: usize,
    _source: PhantomData<S>,
}

impl<M, S, V, P> Loader<M, S, V, P>
where
    M: Migrator<S, V>,
    S: Versioned,
    V: VersionControlled + Default,
    P: Persistor<S>,
{
    pub async fn load(migrator: M, persistor: P, target_version: usize) -> Result<Self> {
        let value = match persistor.load().await? {
            Some(source) => {
                if source.version() > target_version {
                    return Err(Error::StoredVersionNewer {
                        stored: source.version(),
                        supported: target_version,
                    });
                }

                let should_save = source.version() < target_version;
                let value = migrator.migrate(source)?.with_version(target_version);

                if should_save {
                    let stored = encode_for_storage(&migrator, &value, target_version)?;
                    persistor.save(&stored).await?;
                }

                value
            }
            None => V::default().with_version(target_version),
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(value)),
            migrator,
            persistor,
            target_version,
            _source: PhantomData,
        })
    }

    pub async fn cloned(&self) -> V
    where
        V: Clone,
    {
        let guard = self.inner.read().await;
        guard.clone()
    }

    pub async fn get<T>(&self, f: impl FnOnce(&V) -> T) -> T {
        let guard = self.inner.read().await;
        f(&*guard)
    }

    pub async fn set(&self, f: impl FnOnce(&mut V)) -> Result<()> {
        let mut guard = self.inner.write().await;
        f(&mut *guard);
        let stored = encode_for_storage(&self.migrator, &*guard, self.target_version)?;
        self.persistor.save(&stored).await?;
        Ok(())
    }
}

fn encode_for_storage<M, S, V>(migrator: &M, value: &V, target_version: usize) -> Result<S>
where
    M: Migrator<S, V>,
    S: Versioned,
    V: VersionControlled,
{
    let stored = migrator.encode(value)?;
    let stored_version = stored.version();

    if stored_version != target_version {
        return Err(Error::EncodedVersionMismatch {
            encoded: stored_version,
            target: target_version,
        });
    }

    Ok(stored)
}

pub type ProfileLoader<M, S, C, P> = Loader<M, S, Profile<C>, P>;
