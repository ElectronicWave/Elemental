use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoDriver {
    Vanilla,
    Fabric,
    LegacyFabric,
    Babric,
    Quilt,
    Forge,
}

#[derive(Clone, Debug)]
pub struct DemoConfig {
    pub driver: DemoDriver,
    pub storage_root: PathBuf,
    pub instance_name: String,
    pub game_version: String,
    pub loader_version: Option<String>,
}

impl DemoDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::LegacyFabric => "LegacyFabric",
            Self::Babric => "Babric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
        }
    }
}
