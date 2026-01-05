use crate::version::{Migrator, VersionControlled};
use anyhow::Result;

pub struct NoMigrator;
impl<V: VersionControlled> Migrator<V> for NoMigrator {
    fn migrate(&self, value: V, _target_version: usize) -> Result<V> {
        Ok(value)
    }
}
