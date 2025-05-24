use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ModLoader {
    Forge,
    NeoForge,
    Fabric,
    Quilt,
    LiteLoader,
    Cleanroom,
    Datapack,
    Unknown,
}

impl ModLoader {
    pub fn from_name(name: String) -> Self {
        match name.to_lowercase().as_str() {
            "forge" => ModLoader::Forge,
            "neoforge" => ModLoader::NeoForge,
            "fabric" => ModLoader::Fabric,
            "quilt" => ModLoader::Quilt,
            "liteloader" => ModLoader::LiteLoader,
            "cleanroom" => ModLoader::Cleanroom,
            "datapack" => ModLoader::Datapack,
            _ => ModLoader::Unknown,
        }
    }

    pub fn name(&self) -> String {
        match self {
            ModLoader::Forge => "forge".to_owned(),
            ModLoader::NeoForge => "neoforge".to_owned(),
            ModLoader::Fabric => "fabric".to_owned(),
            ModLoader::Quilt => "quilt".to_owned(),
            ModLoader::LiteLoader => "liteloader".to_owned(),
            ModLoader::Cleanroom => "cleanroom".to_owned(),
            ModLoader::Datapack => "datapack".to_owned(),
            ModLoader::Unknown => "unknown".to_owned(),
        }
    }
}
