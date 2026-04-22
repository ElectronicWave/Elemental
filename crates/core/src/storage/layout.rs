use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};

pub trait Layout: Send + Sync {
    type Resource: Send + 'static;
    type ExtendedResource: Send + 'static;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf>;
    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf>;
    fn name(&self) -> &'static str;

    fn try_get_resource(&self, root: &Path, resource: Self::Resource) -> Result<PathBuf>
    where
        Self::Resource: Debug,
    {
        let resource_name = format!("{resource:?}");
        self.get_resource(root, resource).ok_or_else(|| {
            anyhow!(
                "layout '{}' is missing resource {} for '{}'",
                self.name(),
                resource_name,
                root.display()
            )
        })
    }

    fn try_get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Result<PathBuf>
    where
        Self::ExtendedResource: Debug,
    {
        let resource_name = format!("{resource:?}");
        self.get_extended_resource(root, resource).ok_or_else(|| {
            anyhow!(
                "layout '{}' is missing extended resource {} for '{}'",
                self.name(),
                resource_name,
                root.display()
            )
        })
    }
}

pub trait Layoutable<L: Layout> {
    fn layout(&self) -> &L;
    fn root_path(&self) -> &Path;

    fn get_resource(&self, resource: L::Resource) -> Option<PathBuf> {
        self.layout().get_resource(self.root_path(), resource)
    }

    fn try_get_resource(&self, resource: L::Resource) -> Result<PathBuf>
    where
        L::Resource: Debug,
    {
        self.layout().try_get_resource(self.root_path(), resource)
    }

    fn get_existing_resource(&self, resource: L::Resource) -> Option<PathBuf> {
        self.get_resource(resource)
            .and_then(|p| if p.exists() { Some(p) } else { None })
    }

    fn get_extended_resource(&self, resource: L::ExtendedResource) -> Option<PathBuf> {
        self.layout()
            .get_extended_resource(self.root_path(), resource)
    }

    fn try_get_extended_resource(&self, resource: L::ExtendedResource) -> Result<PathBuf>
    where
        L::ExtendedResource: Debug,
    {
        self.layout()
            .try_get_extended_resource(self.root_path(), resource)
    }
}
