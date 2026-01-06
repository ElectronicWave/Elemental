use std::{io::ErrorKind, marker::PhantomData};

use crate::{
    scope::Scope,
    version::{Persistor, VersionControlled},
};
use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::fs::{read_to_string, write};

pub struct NoPersistor;
impl<V: VersionControlled> Persistor<V> for NoPersistor {
    async fn load(&self) -> Result<Option<V>> {
        Ok(Some(V::default()))
    }

    async fn save(&self, _value: &V) -> Result<()> {
        Ok(())
    }
}

pub struct CustomPersistor<
    V: Serialize + DeserializeOwned,
    S, // Always String / buf
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&S) -> Result<V>,
> {
    pub ser: Ser,
    pub de: De,
    pub id: String,
    pub scope: Scope,
    pub suffix: Option<String>,
    _marker: PhantomData<V>,
}

impl<
    V: VersionControlled + Serialize + DeserializeOwned,
    Ser: Fn(&V) -> Result<String>,
    De: Fn(&String) -> Result<V>,
> Persistor<V> for CustomPersistor<V, String, Ser, De>
{
    async fn load(&self) -> Result<Option<V>> {
        let path = self
            .scope
            .get_full_path(&self.id, self.suffix.clone())
            .await?;
        let data = match read_to_string(&path).await {
            Ok(d) => d,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None), // File not found
            Err(e) => return Err(e.into()),
        };
        Ok(Some((self.de)(&data)?))
    }

    async fn save(&self, value: &V) -> Result<()> {
        let data = (self.ser)(value)?;
        let path = self
            .scope
            .get_full_path(&self.id, self.suffix.clone())
            .await?;
        write(path, data).await?;
        Ok(())
    }
}

#[cfg(feature = "json")]
pub fn json_persistor<V: Serialize + DeserializeOwned>(
    id: String,
    scope: Scope,
) -> CustomPersistor<V, String, impl Fn(&V) -> Result<String>, impl Fn(&String) -> Result<V>> {
    CustomPersistor {
        ser: |v: &V| Ok(serde_json::to_string(v)?),
        de: |s: &String| Ok(serde_json::from_str(s)?),
        id,
        scope,
        suffix: Some(".json".to_string()),
        _marker: PhantomData,
    }
}
