use elemental_schema::mojang::piston::PistonMetaLibraries;

use super::merge::merge_libraries_prefer_embedded;

pub fn is_retro_futura_runtime_library(library: &PistonMetaLibraries) -> bool {
    let name = library.name.as_str();
    name.starts_with("org.lwjgl.lwjgl:lwjgl:")
        || name.starts_with("org.lwjgl.lwjgl:lwjgl_util:")
        || name.starts_with("org.lwjgl.lwjgl:lwjgl-platform:")
        || name.starts_with("net.java.jinput:jinput:")
        || name.starts_with("net.java.jinput:jinput-platform:")
        || name.starts_with("net.java.jutils:jutils:")
        || name.starts_with("net.java.dev.jna:platform:")
        || name.starts_with("oshi-project:oshi-core:")
        || name.starts_with("com.ibm.icu:icu4j-core-mojang:")
}

/// Replaces runtime libraries that Cleanroom and Outlands rewrite for legacy
/// Minecraft versions with the embedded versions from their installers.
pub fn merge_retro_futura_libraries(
    base_libraries: Vec<PistonMetaLibraries>,
    embedded_libraries: Vec<PistonMetaLibraries>,
) -> Vec<PistonMetaLibraries> {
    let filtered_base_libraries = base_libraries
        .into_iter()
        .filter(|library| !is_retro_futura_runtime_library(library))
        .collect();
    merge_libraries_prefer_embedded(filtered_base_libraries, embedded_libraries)
}
