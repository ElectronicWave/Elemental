use elemental_infra::SafeRelativePath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionJsonRootResource {
    Assets,
    AssetIndexes(Option<SafeRelativePath>),
    AssetObjects(Option<SafeRelativePath>),
    AssetLogConfigs(Option<SafeRelativePath>),
    Versions(Option<SafeRelativePath>),
    Libraries(Option<SafeRelativePath>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    Elemental(Option<SafeRelativePath>),
}
