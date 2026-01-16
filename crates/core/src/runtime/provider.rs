use std::path::PathBuf;

pub trait RuntimeProvider {
    fn list(&self) -> Vec<PathBuf>;

    #[inline(always)]
    fn name(&self) -> &'static str {
        return "Provider";
    }

    fn box_default() -> Box<Self>
    where
        Self: Sized + Default,
    {
        Box::new(Self::default())
    }
}

/// Re-export providers
pub use super::providers::{
    envjavahome::EnvJavaHomeProvider, envpath::EnvPathProvider, pm::PackageManagerProvider,
    registry::RegistryProvider,
};

pub fn all_providers() -> Vec<Box<dyn RuntimeProvider>> {
    vec![
        RegistryProvider::box_default(),
        EnvPathProvider::box_default(),
        PackageManagerProvider::box_default(),
        EnvJavaHomeProvider::box_default(),
    ]
}
