use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::storage::error::{Error, Result};

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
        self.get_resource(root, resource)
            .ok_or_else(|| Error::ResourceNotFound {
                layout: self.name(),
                resource: resource_name,
                root: root.to_path_buf(),
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
        self.get_extended_resource(root, resource)
            .ok_or_else(|| Error::ResourceNotFound {
                layout: self.name(),
                resource: resource_name,
                root: root.to_path_buf(),
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
        self.get_resource(resource).filter(|path| path.exists())
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

impl Layout for () {
    type Resource = ();
    type ExtendedResource = ();

    fn get_resource(&self, _root: &Path, _resource: Self::Resource) -> Option<PathBuf> {
        None
    }

    fn get_extended_resource(
        &self,
        _root: &Path,
        _resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        None
    }

    fn name(&self) -> &'static str {
        "empty"
    }
}
