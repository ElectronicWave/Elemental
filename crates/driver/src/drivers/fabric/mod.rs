use crate::catalog::LoaderMetaCatalog;

use self::source::FabricEndpoints;

pub mod driver;
mod flavors;
pub mod source;

pub type FabricCatalog = LoaderMetaCatalog<FabricEndpoints>;
