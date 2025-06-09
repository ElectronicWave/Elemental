use std::io::Result;
use std::path::absolute;

pub struct VersionStorage {
    pub root: String,
}

impl VersionStorage {
    pub fn new(root: impl Into<String>) -> Result<Self> {
        Ok(Self {
            root: absolute(root.into())?.to_string_lossy().to_string(),
        })
    }
    
}
