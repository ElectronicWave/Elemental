use std::collections::HashMap;

use elemental_schema::mojang::piston::PistonMetaLibrariesDownloads;

use crate::{
    error::Result,
    families::version_json::VersionJsonPlatform,
    maven::{MavenCoordinate, maven_library_downloads},
};

pub(super) fn profile_library_downloads_from_maven(
    name: &str,
    url: &str,
    natives: Option<&HashMap<String, String>>,
) -> Result<PistonMetaLibrariesDownloads> {
    let coordinate = MavenCoordinate::parse_labeled(name, "maven artifact")?;
    maven_library_downloads(
        &coordinate,
        url,
        None,
        None,
        natives,
        VersionJsonPlatform::current().arch_bits(),
    )
}
