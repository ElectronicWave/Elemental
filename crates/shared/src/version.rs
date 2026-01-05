use anyhow::Result;

pub trait VersionControlled {
    fn version(&self) -> usize;
    fn is_up_to_date(&self, latest_version: usize) -> bool {
        self.version() >= latest_version
    }
}
pub trait Migrator<V: VersionControlled> {
    fn migrate(&self, value: V, target_version: usize) -> Result<V>;
}

pub trait Persistor<V: VersionControlled> {
    fn save(&self, value: &V) -> Result<()>;
    fn load(&self) -> Result<V>
    where
        Self: Sized;
}
