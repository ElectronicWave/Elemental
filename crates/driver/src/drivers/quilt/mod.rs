use crate::catalog::LoaderMetaCatalog;

use self::source::QuiltEndpoints;

pub mod driver;
pub mod source;

pub type QuiltCatalog = LoaderMetaCatalog<QuiltEndpoints>;
