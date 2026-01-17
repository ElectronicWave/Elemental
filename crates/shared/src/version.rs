use anyhow::Result;

pub trait VersionControlled: Default {
    fn version(&self) -> usize;
    fn is_up_to_date(&self, latest_version: usize) -> bool {
        self.version() >= latest_version
    }
}
pub trait Migrator<V: VersionControlled> {
    fn migrate(&self, value: V, target_version: usize) -> Result<V>;
}

pub trait Persistor<V: VersionControlled> {
    fn save(&self, value: &V) -> impl Future<Output = Result<()>>;
    fn load(&self) -> impl Future<Output = Result<Option<V>>>
    where
        Self: Sized;
}
