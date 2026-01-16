use crate::{
    scope::Scope,
    version::{Persistor, VersionControlled},
};
use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::{io::ErrorKind, marker::PhantomData, path::PathBuf};
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

pub trait PersistorIO<S> {
    fn read(path: PathBuf) -> impl Future<Output = Result<Option<S>>>;
    fn write(path: PathBuf, data: S) -> impl Future<Output = Result<()>>;
}

pub struct AsyncFileStringIO;

impl PersistorIO<String> for AsyncFileStringIO {
    async fn read(path: PathBuf) -> Result<Option<String>> {
        match read_to_string(&path).await {
            Ok(d) => Ok(Some(d)),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None), // File not found
            Err(e) => return Err(e.into()),
        }
    }

    async fn write(path: PathBuf, data: String) -> Result<()> {
        write(&path, data).await?;
        Ok(())
    }
}

pub struct CustomPersistor<
    V: Serialize + DeserializeOwned,
    S,
    IO: PersistorIO<S>,
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&S) -> Result<V>,
> {
    pub ser: Ser,
    pub de: De,
    pub id: String,
    pub scope: Scope,
    pub suffix: Option<String>,
    _marker: PhantomData<(IO, V)>,
}

impl<V, S, IO, Ser, De> CustomPersistor<V, S, IO, Ser, De>
where
    V: Serialize + DeserializeOwned,
    IO: PersistorIO<S>,
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&S) -> Result<V>,
{
    pub fn new(ser: Ser, de: De, id: String, scope: Scope, suffix: Option<String>) -> Self {
        Self {
            ser: ser,
            de: de,
            id: id,
            scope: scope,
            suffix: suffix,
            _marker: PhantomData,
        }
    }
}

impl<V, S, IO, Ser, De> Persistor<V> for CustomPersistor<V, S, IO, Ser, De>
where
    V: VersionControlled + Serialize + DeserializeOwned,
    IO: PersistorIO<S>,
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&S) -> Result<V>,
{
    async fn load(&self) -> Result<Option<V>> {
        let path = self
            .scope
            .get_full_path(&self.id, self.suffix.clone())
            .await?;
        let data = IO::read(path).await?;
        match data {
            Some(d) => Ok(Some((self.de)(&d)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, value: &V) -> Result<()> {
        let data = (self.ser)(value)?;
        let path = self
            .scope
            .get_full_path(&self.id, self.suffix.clone())
            .await?;
        IO::write(path, data).await?;
        Ok(())
    }
}

#[cfg(feature = "json")]
pub fn json_persistor<V: Serialize + DeserializeOwned>(
    id: String,
    scope: Scope,
) -> CustomPersistor<
    V,
    String,
    AsyncFileStringIO,
    impl Fn(&V) -> Result<String>,
    impl Fn(&String) -> Result<V>,
> {
    CustomPersistor::new(
        |v: &V| Ok(serde_json::to_string(v)?),
        |s: &String| Ok(serde_json::from_str(s)?),
        id,
        scope,
        Some(".json".to_string()),
    )
}

#[cfg(feature = "toml")]
pub fn toml_persistor<V: Serialize + DeserializeOwned>(
    id: String,
    scope: Scope,
) -> CustomPersistor<
    V,
    String,
    AsyncFileStringIO,
    impl Fn(&V) -> Result<String>,
    impl Fn(&String) -> Result<V>,
> {
    CustomPersistor::new(
        |v: &V| Ok(toml::to_string(v)?),
        |s: &String| Ok(toml::from_str(s)?),
        id,
        scope,
        Some(".toml".to_string()),
    )
}
