use super::common::FlavorBehavior;
use crate::families::version_json::{LibraryReplacementFamily, ProfileMergeBehavior};

pub(super) static BEHAVIOR: BabricBehavior = BabricBehavior;

pub(super) struct BabricBehavior;

impl FlavorBehavior for BabricBehavior {}

impl ProfileMergeBehavior for BabricBehavior {
    fn profile_replacement_family(&self, library_name: &str) -> Option<LibraryReplacementFamily> {
        if library_name.starts_with("org.ow2.asm:") {
            return Some(LibraryReplacementFamily::Asm);
        }

        if library_name.starts_with("org.lwjgl.lwjgl:lwjgl")
            || library_name.starts_with("org.lwjgl.lwjgl:lwjgl_util")
            || library_name.starts_with("org.lwjgl.lwjgl:lwjgl-platform")
        {
            return Some(LibraryReplacementFamily::Lwjgl2);
        }

        None
    }

    fn replaced_base_family(&self, library_name: &str) -> Option<LibraryReplacementFamily> {
        if library_name.starts_with("org.ow2.asm:asm-all:") {
            return Some(LibraryReplacementFamily::Asm);
        }

        if library_name.starts_with("org.lwjgl.lwjgl:lwjgl:")
            || library_name.starts_with("org.lwjgl.lwjgl:lwjgl_util:")
            || library_name.starts_with("org.lwjgl.lwjgl:lwjgl-platform:")
        {
            return Some(LibraryReplacementFamily::Lwjgl2);
        }

        None
    }
}
