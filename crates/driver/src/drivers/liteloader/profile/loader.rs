use elemental_infra::SafeRelativePath;
use elemental_schema::fabric::ProfileLibrary;

use crate::{error::Result, maven::MavenNotation};

use super::super::manifest::{LiteLoaderRepositoryStream, LiteLoaderSelectedBuild};
use super::super::source::LiteLoaderEndpoints;
use super::library::{profile_library_with_repository, rewritten_repository};

pub(super) async fn build_loader_library(
    client: &reqwest::Client,
    endpoints: &LiteLoaderEndpoints,
    selected: &LiteLoaderSelectedBuild,
) -> Result<ProfileLibrary> {
    let notation = format!("com.mumfrey:liteloader:{}", selected.build.version);
    let parsed = MavenNotation::parse(notation.as_str(), "liteloader library")?;
    let repository = rewritten_repository(
        endpoints,
        selected.repository.url.as_str(),
        "liteloader repository",
    )?;
    let artifact = if selected.repository.stream == LiteLoaderRepositoryStream::Snapshot {
        repository
            .resolve_snapshot_artifact(client, &parsed)
            .await?
    } else {
        repository.artifact_download(SafeRelativePath::parse(format!(
            "{}/{}",
            parsed.version_directory_path()?.as_str(),
            selected.build.file.as_str()
        ))?)?
    };

    Ok(profile_library_with_repository(
        notation,
        &repository,
        Some(artifact),
    ))
}
