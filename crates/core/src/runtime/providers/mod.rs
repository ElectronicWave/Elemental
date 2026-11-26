pub mod custom;
pub mod envjavahome;
pub mod envpath;
pub mod pm;
pub mod registry;

use self::{
    envjavahome::EnvJavaHomeProvider, envpath::EnvPathProvider, pm::PackageManagerProvider,
    registry::RegistryProvider,
};
use super::provider::{RuntimeProvider, RuntimeProviderList};

pub fn default_providers() -> RuntimeProviderList {
    vec![
        RegistryProvider::arc_default(),
        EnvPathProvider::arc_default(),
        PackageManagerProvider::arc_default(),
        EnvJavaHomeProvider::arc_default(),
    ]
}
