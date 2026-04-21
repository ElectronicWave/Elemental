use anyhow::Result;
use elemental_schema::mojang::piston::PistonMetaLibraries;

pub fn normalize_library_urls<F>(
    libraries: Vec<PistonMetaLibraries>,
    artifact_url: F,
) -> Result<Vec<PistonMetaLibraries>>
where
    F: Fn(&str) -> Result<String>,
{
    let mut normalized = Vec::with_capacity(libraries.len());

    for mut library in libraries {
        if let Some(artifact) = &mut library.downloads.artifact
            && artifact.url.trim().is_empty()
        {
            artifact.url = artifact_url(artifact.path.as_str())?;
        }

        if let Some(classifiers) = &mut library.downloads.classifiers {
            for artifact in classifiers.values_mut() {
                if artifact.url.trim().is_empty() {
                    artifact.url = artifact_url(artifact.path.as_str())?;
                }
            }
        }

        normalized.push(library);
    }

    Ok(normalized)
}
