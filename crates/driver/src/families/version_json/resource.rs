use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionJsonRootResource {
    Assets,
    AssetIndexes(Option<String>),
    AssetObjects(Option<String>),
    AssetLogConfigs(Option<String>),
    Versions(Option<String>),
    Libraries(Option<PathBuf>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionJsonInstanceResource {
    Metadata,
    Jar,
    Natives,
    Logs,
    Configs,
    ShaderPacks,
    ResourcePacks,
    Saves,
    Mods,
}
