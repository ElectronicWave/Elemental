#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resource {
    AssetsIndexes,
    AssetsObjects,
    AssetsLogConfigs,
    Versions,
    Logs,
    Configs,
    ShaderPacks,
    ResourcePacks,
    Saves,
    Libraries,
    Natives,
    Mods,
    Dot,
    Subdir(String),
    Custom(String),
}

impl Resource {
    pub fn subdir(path: impl Into<String>) -> Self {
        Self::Subdir(path.into())
    }

    pub fn custom(path: impl Into<String>) -> Self {
        Self::Custom(path.into())
    }
}
