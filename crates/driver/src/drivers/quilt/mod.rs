use crate::families::version_json::LoaderMetaCatalog;

use self::source::QuiltEndpoints;

pub mod driver;
pub mod source;

pub type QuiltCatalog = LoaderMetaCatalog<QuiltEndpoints>;
