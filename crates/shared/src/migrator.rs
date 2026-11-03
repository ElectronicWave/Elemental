use crate::{
    error::Result,
    version::{VersionControlled, Versioned},
};

pub trait Migrator<Stored: Versioned, Current: VersionControlled> {
    fn migrate(&self, stored: Stored) -> Result<Current>;
    fn encode(&self, current: &Current) -> Result<Stored>;
}

pub struct NoMigrator;

impl<T: VersionControlled + Clone> Migrator<T, T> for NoMigrator {
    fn migrate(&self, stored: T) -> Result<T> {
        Ok(stored)
    }

    fn encode(&self, current: &T) -> Result<T> {
        Ok(current.clone())
    }
}
