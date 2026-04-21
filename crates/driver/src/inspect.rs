use std::{fs::File, path::PathBuf};

use anyhow::Result;
use elemental_core::storage::{Storage, layout::Layout};

use crate::{
    driver::{Driver, InstalledDriver},
    families::version_json::PistonMetaData,
};

#[derive(Debug, Clone)]
pub struct InstanceProbe<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub metadata_path: Option<PathBuf>,
    pub metadata: Option<PistonMetaData>,
    pub main_class: Option<String>,
    pub library_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InstalledInstance<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub driver: InstalledDriver,
}

impl<L: Layout, VL: Layout> InstanceProbe<L, VL> {
    pub fn collect(storage: Storage<VL, Storage<L>>) -> Result<Self> {
        let metadata_path = detect_metadata_path(&storage)?;
        let metadata: Option<PistonMetaData> = match metadata_path.as_ref() {
            Some(path) => Some(serde_json::from_reader(File::open(path)?)?),
            None => None,
        };
        let main_class = metadata
            .as_ref()
            .map(|metadata| metadata.main_class.clone());
        let library_names = metadata
            .as_ref()
            .map(|metadata| {
                metadata
                    .libraries
                    .iter()
                    .map(|library| library.name.clone())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        Ok(Self {
            storage,
            metadata_path,
            metadata,
            main_class,
            library_names,
        })
    }
}

pub async fn inspect_instance<L, VL>(
    storage: Storage<VL, Storage<L>>,
    drivers: &[&dyn Driver<L, VL>],
) -> Result<Option<InstalledInstance<L, VL>>>
where
    L: Layout + Clone,
    VL: Layout + Clone,
{
    let probe = InstanceProbe::collect(storage.clone())?;
    for driver in drivers {
        if let Some(installed) = driver.inspect(&probe).await? {
            return Ok(Some(InstalledInstance {
                storage,
                driver: installed,
            }));
        }
    }

    Ok(None)
}

fn detect_metadata_path<L: Layout, VL: Layout>(
    storage: &Storage<VL, Storage<L>>,
) -> Result<Option<PathBuf>> {
    let preferred: Option<PathBuf> = storage
        .name()
        .map(|name| storage.path.join(format!("{name}.json")))
        .filter(|path| path.exists());
    if preferred.is_some() {
        return Ok(preferred);
    }

    let mut json_files = Vec::new();
    if storage.path.exists() {
        for entry in storage.path.read_dir()? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                json_files.push(entry.path());
            }
        }
    }

    Ok(if json_files.len() == 1 {
        json_files.into_iter().next()
    } else {
        None
    })
}
