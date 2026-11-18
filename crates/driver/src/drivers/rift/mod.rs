use crate::inspect::{LibraryPrefixSet, ProfileIdPattern};

pub(crate) const RIFT_PROFILE_ID: ProfileIdPattern = ProfileIdPattern::new("-rift-");
pub(crate) const RIFT_LIBRARY_MARKERS: LibraryPrefixSet =
    LibraryPrefixSet::new(&["org.dimdev:rift:", "com.github.Chocohead:Rift:"]);
pub(crate) const RIFT_VERSION_LIBRARIES: LibraryPrefixSet =
    LibraryPrefixSet::new(&["org.dimdev:rift:"]);
pub(crate) const RIFT_STALE_MARKERS: LibraryPrefixSet =
    LibraryPrefixSet::new(&["com.github.Chocohead:Rift:"]);

pub mod catalog;
pub mod driver;
pub mod source;
