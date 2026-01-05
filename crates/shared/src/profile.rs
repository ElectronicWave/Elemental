use crate::{config::Config, migrate::BackwardsCompatible};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Profile {
    /// Profile name
    pub name: String,
    /// Profile configuration data
    pub config: Config,
    pub version: usize,
}

impl Profile {
    pub fn migrate<M: BackwardsCompatible>(
        &self,
        migrator: &M,
        target_version: usize,
    ) -> Result<()> {
        migrator.migrate(target_version)
    }

    pub fn is_up_to_date(&self, latest_version: usize) -> bool {
        self.version >= latest_version
    }
}
