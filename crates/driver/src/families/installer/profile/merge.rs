use elemental_schema::mojang::piston::{
    PistonMetaArguments, PistonMetaAssetIndex, PistonMetaData, PistonMetaDownloads,
    PistonMetaJavaVersion, PistonMetaLibraries, PistonMetaLogging,
};
use serde::Deserialize;

use crate::{
    error::{Error, Result},
    families::version_json::merge_piston_meta_arguments,
};

#[derive(Debug, Clone, Deserialize)]
struct EmbeddedVersionData {
    #[serde(default)]
    arguments: Option<PistonMetaArguments>,
    #[serde(rename = "minecraftArguments", default)]
    minecraft_arguments: Option<String>,
    #[serde(rename = "inheritsFrom", default)]
    inherits_from: Option<String>,
    #[serde(rename = "assetIndex", default)]
    asset_index: Option<PistonMetaAssetIndex>,
    #[serde(default)]
    assets: Option<String>,
    #[serde(rename = "complianceLevel", default)]
    compliance_level: Option<usize>,
    #[serde(default)]
    downloads: Option<PistonMetaDownloads>,
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "javaVersion", default)]
    java_version: Option<PistonMetaJavaVersion>,
    #[serde(default)]
    libraries: Vec<PistonMetaLibraries>,
    #[serde(default)]
    logging: Option<PistonMetaLogging>,
    #[serde(rename = "mainClass", default)]
    main_class: Option<String>,
    #[serde(rename = "minimumLauncherVersion", default)]
    minimum_launcher_version: Option<usize>,
    #[serde(rename = "type", default)]
    release_type: Option<String>,
    #[serde(default)]
    time: Option<String>,
    #[serde(rename = "releaseTime", default)]
    release_time: Option<String>,
}

pub(super) fn merge_embedded_version<F, M>(
    base_metadata: PistonMetaData,
    embedded_version: &serde_json::Value,
    normalize_libraries: F,
    merge_libraries: M,
    family_name: &str,
) -> Result<PistonMetaData>
where
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
    M: Fn(Vec<PistonMetaLibraries>, Vec<PistonMetaLibraries>) -> Vec<PistonMetaLibraries>,
{
    let mut embedded = serde_json::from_value::<EmbeddedVersionData>(embedded_version.clone())
        .map_err(|source| Error::DecodeEmbeddedVersion {
            family_name: family_name.to_owned(),
            source,
        })?;
    embedded.libraries = normalize_libraries(embedded.libraries)?;
    let (arguments, minecraft_arguments) = merge_embedded_arguments(
        base_metadata.arguments.clone(),
        base_metadata.minecraft_arguments.clone(),
        embedded.arguments,
        embedded.minecraft_arguments,
    )?;

    Ok(PistonMetaData {
        arguments,
        minecraft_arguments,
        inherits_from: embedded
            .inherits_from
            .or(base_metadata.inherits_from.clone()),
        asset_index: embedded.asset_index.unwrap_or(base_metadata.asset_index),
        assets: embedded.assets.unwrap_or(base_metadata.assets),
        compliance_level: embedded
            .compliance_level
            .unwrap_or(base_metadata.compliance_level),
        downloads: embedded.downloads.unwrap_or(base_metadata.downloads),
        id: embedded.id.unwrap_or(base_metadata.id),
        java_version: embedded.java_version.unwrap_or(base_metadata.java_version),
        libraries: merge_libraries(base_metadata.libraries, embedded.libraries),
        logging: merge_embedded_logging(base_metadata.logging, embedded.logging),
        main_class: embedded.main_class.unwrap_or(base_metadata.main_class),
        minimum_launcher_version: embedded
            .minimum_launcher_version
            .unwrap_or(base_metadata.minimum_launcher_version),
        release_type: embedded.release_type.unwrap_or(base_metadata.release_type),
        time: embedded.time.unwrap_or(base_metadata.time),
        release_time: embedded.release_time.unwrap_or(base_metadata.release_time),
    })
}

fn merge_embedded_arguments(
    base_arguments: Option<PistonMetaArguments>,
    base_minecraft_arguments: Option<String>,
    embedded_arguments: Option<PistonMetaArguments>,
    embedded_minecraft_arguments: Option<String>,
) -> Result<(Option<PistonMetaArguments>, Option<String>)> {
    if embedded_minecraft_arguments.is_some() {
        return Ok((None, embedded_minecraft_arguments));
    }

    Ok((
        merge_piston_meta_arguments(base_arguments, base_minecraft_arguments, embedded_arguments)?,
        None,
    ))
}

fn merge_embedded_logging(
    base_logging: Option<PistonMetaLogging>,
    embedded_logging: Option<PistonMetaLogging>,
) -> Option<PistonMetaLogging> {
    match (base_logging, embedded_logging) {
        (Some(base), Some(embedded)) => Some(PistonMetaLogging {
            client: embedded.client.or(base.client),
        }),
        (base, embedded) => embedded.or(base),
    }
}
