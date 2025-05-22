pub enum ModLoader {
    Forge,
    NeoForge,
    Fabric,
    Quilt,
    LiteLoader,
    Cleanroom,
    Datapack
}

pub fn to_loader(name: String) -> ModLoader {
    match name.to_lowercase().as_str() {
        "forge" => ModLoader::Forge,
        "neoforge" => ModLoader::NeoForge,
        "fabric" => ModLoader::Fabric,
        "quilt" => ModLoader::Quilt,
        "liteloader" => ModLoader::LiteLoader,
        "cleanroom" => ModLoader::Cleanroom,
        "datapack" => ModLoader::Datapack,
        _ => panic!("Unknown mod loader: {}", name)
    }
}

pub fn to_name(loader: ModLoader) -> String {
    match loader {
        ModLoader::Forge => "forge".to_string(),
        ModLoader::NeoForge => "neoforge".to_string(),
        ModLoader::Fabric => "fabric".to_string(),
        ModLoader::Quilt => "quilt".to_string(),
        ModLoader::LiteLoader => "liteloader".to_string(),
        ModLoader::Cleanroom => "cleanroom".to_string(),
        ModLoader::Datapack => "datapack".to_string()
    }
}