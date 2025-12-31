use anyhow::Result;

pub trait Persistence {
    fn save(&self, path: &str) -> Result<()>;
    fn load(path: &str) -> Result<Self>
    where
        Self: Sized;
}
