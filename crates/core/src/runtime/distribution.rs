use std::path::PathBuf;

use crate::runtime::provider::RuntimeProvider;
#[derive(Debug)]
pub struct DistributionReleaseData {
    /// java.vm.vendor e.g. Eclipse Adoptium
    pub implementor: Option<String>,
    //? impl version may need ?
    /// os.arch e.g. x86_64
    pub architecture: Option<String>,
    /// java.specification.version e.g. 1.8
    pub major_version: Option<String>,
    /// java.version e.g. 1.8.0_452
    pub jre_version: Option<String>,
    /// java.vm.version e.g. 24.0.1+9
    pub jvm_version: Option<String>,
}

#[derive(Debug)]
pub struct Distribution {
    // Release Data (when available)
    pub release: Option<DistributionReleaseData>,

    // Physical Data
    /// Path to the runtime installation
    pub path: PathBuf,

    // Discovery Data
    /// Elemental Provider used to discover this runtime
    pub provider: &'static str,
}

impl DistributionReleaseData {
    pub fn parse_from_commandline() -> Self {
        todo!()
    }

    pub fn parse_from_release() -> Self {
        todo!()
    }

    pub fn is_lts(&self) -> bool {
        match &self.major_version {
            Some(ver) => match ver.as_str() {
                "8" | "11" | "17" | "21" => true,
                _ => false,
            },
            None => false,
        }
    }
}

impl Distribution {
    pub fn build_from_root(root: PathBuf, provider: &'static str) -> Self {
        Self {
            release: None,
            path: root,
            provider,
        }
    }

    pub fn from_providers<O>(providers: Vec<Box<dyn RuntimeProvider>>) -> O
    where
        O: FromIterator<Self>,
    {
        providers
            .into_iter()
            .flat_map(|provider| {
                let name = provider.name();
                provider
                    .list()
                    .into_iter()
                    .map(|path| Distribution::build_from_root(path, name))
            })
            .collect()
    }
}

#[test]
fn test_distribution_build() {
    use super::provider::{EnvPathProvider, RegistryProvider};
    let _: Vec<_> = Distribution::from_providers(vec![
        RegistryProvider::box_default(),
        EnvPathProvider::box_default(),
    ]);
}
