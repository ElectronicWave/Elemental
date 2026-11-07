use elemental_core::storage::{Storage, layout::Layout};

use crate::{
    driver::{Driver, InstalledDriver},
    error::Result,
    families::version_json::{
        VersionJsonGameStorageExt, VersionJsonInstanceLayout, VersionJsonRootLayout,
    },
};

use super::InstanceProbe;

#[derive(Debug, Clone)]
pub struct InstalledInstance<L: Layout, VL: Layout> {
    pub storage: Storage<VL, Storage<L>>,
    pub driver: InstalledDriver,
}

impl<L: Layout + Clone, VL: VersionJsonInstanceLayout + Clone> InstalledInstance<L, VL> {
    pub async fn detect(
        storage: Storage<VL, Storage<L>>,
        drivers: &[&dyn Driver<L, VL>],
    ) -> Result<Option<Self>> {
        let probe = InstanceProbe::collect(storage.clone())?;
        for driver in drivers {
            if let Some(installed) = driver.inspect(&probe).await? {
                return Ok(Some(Self {
                    storage,
                    driver: installed,
                }));
            }
        }

        Ok(None)
    }
}

impl<L, VL> InstalledInstance<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn detect_all(
        storage: &Storage<L>,
        version_layout: VL,
        drivers: &[&dyn Driver<L, VL>],
    ) -> Result<Vec<Self>> {
        let mut instances = Vec::new();

        for instance in storage.instances(version_layout)? {
            if let Some(installed) = Self::detect(instance, drivers).await? {
                instances.push(installed);
            }
        }

        Ok(instances)
    }
}
