use crate::{config::Config, version::VersionControlled};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Profile {
    /// Profile name
    pub name: String,
    /// Profile configuration data
    pub config: Config,
    pub version: usize,
}

impl VersionControlled for Profile {
    #[inline(always)]
    fn version(&self) -> usize {
        self.version
    }
}
