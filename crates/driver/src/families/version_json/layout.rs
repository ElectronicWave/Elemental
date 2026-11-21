use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use elemental_core::storage::{Storage, layout::Layout};

use super::resource::{VersionJsonInstanceResource, VersionJsonRootResource};

mod instance;
mod root;

#[derive(Debug, Clone, Default)]
pub struct BaseRootLayout;

#[derive(Debug, Clone, Default)]
pub struct BaseInstanceLayout;

pub trait VersionJsonRootLayout: Layout<Resource = VersionJsonRootResource> {}

pub trait VersionJsonInstanceLayout: Layout<Resource = VersionJsonInstanceResource> {}

impl<L> VersionJsonRootLayout for L where L: Layout<Resource = VersionJsonRootResource> {}
impl<L> VersionJsonInstanceLayout for L where L: Layout<Resource = VersionJsonInstanceResource> {}

trait DynRootLayout: Send + Sync {
    fn get_resource(&self, root: &Path, resource: VersionJsonRootResource) -> Option<PathBuf>;
    fn name(&self) -> &'static str;
}

impl<L> DynRootLayout for L
where
    L: VersionJsonRootLayout,
{
    fn get_resource(&self, root: &Path, resource: VersionJsonRootResource) -> Option<PathBuf> {
        Layout::get_resource(self, root, resource)
    }

    fn name(&self) -> &'static str {
        Layout::name(self)
    }
}

trait DynInstanceLayout: Send + Sync {
    fn get_resource(&self, root: &Path, resource: VersionJsonInstanceResource) -> Option<PathBuf>;
    fn name(&self) -> &'static str;
}

impl<VL> DynInstanceLayout for VL
where
    VL: VersionJsonInstanceLayout,
{
    fn get_resource(&self, root: &Path, resource: VersionJsonInstanceResource) -> Option<PathBuf> {
        Layout::get_resource(self, root, resource)
    }

    fn name(&self) -> &'static str {
        Layout::name(self)
    }
}

/// Type-erased [`VersionJsonRootLayout`] shared across driver and launcher boundaries.
#[derive(Clone)]
pub struct SharedRootLayout(Arc<dyn DynRootLayout>);

impl SharedRootLayout {
    pub fn new<L>(layout: L) -> Self
    where
        L: VersionJsonRootLayout + 'static,
    {
        Self(Arc::new(layout))
    }
}

impl Default for SharedRootLayout {
    fn default() -> Self {
        Self::new(BaseRootLayout)
    }
}

impl fmt::Debug for SharedRootLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SharedRootLayout")
            .field(&self.0.name())
            .finish()
    }
}

impl Layout for SharedRootLayout {
    type Resource = VersionJsonRootResource;
    type ExtendedResource = VersionJsonRootResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        self.0.get_resource(root, resource)
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.0.get_resource(root, resource)
    }

    fn name(&self) -> &'static str {
        self.0.name()
    }
}

/// Type-erased [`VersionJsonInstanceLayout`] shared across driver and launcher boundaries.
#[derive(Clone)]
pub struct SharedInstanceLayout(Arc<dyn DynInstanceLayout>);

impl SharedInstanceLayout {
    pub fn new<VL>(layout: VL) -> Self
    where
        VL: VersionJsonInstanceLayout + 'static,
    {
        Self(Arc::new(layout))
    }
}

impl Default for SharedInstanceLayout {
    fn default() -> Self {
        Self::new(BaseInstanceLayout)
    }
}

impl fmt::Debug for SharedInstanceLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SharedInstanceLayout")
            .field(&self.0.name())
            .finish()
    }
}

impl Layout for SharedInstanceLayout {
    type Resource = VersionJsonInstanceResource;
    type ExtendedResource = VersionJsonInstanceResource;

    fn get_resource(&self, root: &Path, resource: Self::Resource) -> Option<PathBuf> {
        self.0.get_resource(root, resource)
    }

    fn get_extended_resource(
        &self,
        root: &Path,
        resource: Self::ExtendedResource,
    ) -> Option<PathBuf> {
        self.0.get_resource(root, resource)
    }

    fn name(&self) -> &'static str {
        self.0.name()
    }
}

pub type GameStorage = Storage<SharedRootLayout>;
pub type InstanceStorage = Storage<SharedInstanceLayout, GameStorage>;
