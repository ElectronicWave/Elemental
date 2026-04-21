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
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
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
    D: ?Sized,
    IO: PersistorIO<S>,
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&D) -> Result<V>,
> {
    pub ser: Ser,
    pub de: De,
    pub id: String,
    pub scope: Scope,
    pub suffix: Option<String>,
    _marker: PhantomData<(IO, V, S, *const D)>,
}

impl<V, S, D, IO, Ser, De> CustomPersistor<V, S, D, IO, Ser, De>
where
    V: Serialize + DeserializeOwned,
    D: ?Sized,
    IO: PersistorIO<S>,
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&D) -> Result<V>,
    S: AsRef<D>,
{
    pub fn new(ser: Ser, de: De, id: String, scope: Scope, suffix: Option<String>) -> Self {
        Self {
            ser,
            de,
            id,
            scope,
            suffix,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "json")]
type JsonFilePersistor<V> = CustomPersistor<
    V,
    String,
    str,
    AsyncFileStringIO,
    fn(&V) -> Result<String>,
    fn(&str) -> Result<V>,
>;

#[cfg(feature = "json")]
fn serialize_json<V: Serialize>(value: &V) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

#[cfg(feature = "json")]
fn deserialize_json<V: DeserializeOwned>(value: &str) -> Result<V> {
    Ok(serde_json::from_str(value)?)
}

impl<V, S, D, IO, Ser, De> Persistor<V> for CustomPersistor<V, S, D, IO, Ser, De>
where
    V: VersionControlled + Serialize + DeserializeOwned,
    D: ?Sized,
    IO: PersistorIO<S>,
    Ser: Fn(&V) -> Result<S>,
    De: Fn(&D) -> Result<V>,
    S: AsRef<D>,
{
    async fn load(&self) -> Result<Option<V>> {
        let path = self
            .scope
            .get_full_path(&self.id, self.suffix.clone())
            .await?;
        let data = IO::read(path).await?;
        match data {
            Some(d) => Ok(Some((self.de)(d.as_ref())?)),
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
) -> JsonFilePersistor<V> {
    CustomPersistor::new(
        serialize_json::<V>,
        deserialize_json::<V>,
        id,
        scope,
        Some("json".to_string()),
    )
}

#[cfg(feature = "toml")]
type TomlFilePersistor<V> = CustomPersistor<
    V,
    String,
    str,
    AsyncFileStringIO,
    fn(&V) -> Result<String>,
    fn(&str) -> Result<V>,
>;

#[cfg(feature = "toml")]
fn serialize_toml<V: Serialize>(value: &V) -> Result<String> {
    Ok(toml::to_string(value)?)
}

#[cfg(feature = "toml")]
fn deserialize_toml<V: DeserializeOwned>(value: &str) -> Result<V> {
    Ok(toml::from_str(value)?)
}

#[cfg(feature = "toml")]
pub fn toml_persistor<V: Serialize + DeserializeOwned>(
    id: String,
    scope: Scope,
) -> TomlFilePersistor<V> {
    CustomPersistor::new(
        serialize_toml::<V>,
        deserialize_toml::<V>,
        id,
        scope,
        Some("toml".to_string()),
    )
}
