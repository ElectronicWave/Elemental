use elemental_core::storage::layout::Layout;

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
