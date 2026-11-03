use crate::error::Result;

pub trait Versioned {
    fn version(&self) -> usize;
}

pub trait VersionControlled: Versioned {
    fn with_version(self, version: usize) -> Self
    where
        Self: Sized;
}

pub trait Persistor<V> {
    fn save(&self, value: &V) -> impl Future<Output = Result<()>>;
    fn load(&self) -> impl Future<Output = Result<Option<V>>>
    where
        Self: Sized;
}
