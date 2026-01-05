use crate::version::{Persistor, VersionControlled};
use anyhow::Result;

pub struct NoPersistor;
impl<V: VersionControlled> Persistor<V> for NoPersistor {
    fn load(&self) -> Result<Option<V>> {
        Ok(Some(V::default()))
    }

    fn save(&self, _value: &V) -> anyhow::Result<()> {
        Ok(())
    }
}
