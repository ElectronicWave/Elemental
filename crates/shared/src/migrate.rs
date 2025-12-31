use anyhow::Result;

pub trait BackwardsCompatible {
    fn migrate(&self, target_version: usize) -> Result<()>;
}
