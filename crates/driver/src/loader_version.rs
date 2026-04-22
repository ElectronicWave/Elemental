use elemental_core::version::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoaderVersionTag;

pub type LoaderVersionId = Version<LoaderVersionTag>;
