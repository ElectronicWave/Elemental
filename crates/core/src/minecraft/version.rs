use crate::version::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MinecraftVersionTag;

pub type MinecraftVersionId = Version<MinecraftVersionTag>;
