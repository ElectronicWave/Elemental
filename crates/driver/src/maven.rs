use elemental_core::minecraft::MinecraftVersionId;
use elemental_schema::forge::MavenMetadataBody;
use quick_xml::de::from_str;

use crate::error::{Error, Result};
use crate::http::fetch_text;

pub fn artifact_path(notation: &str, invalid_label: &str) -> Result<String> {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    let segments = coordinates.split(':').collect::<Vec<&str>>();

    let (group, artifact, version, classifier) = match segments.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] => (*group, *artifact, *version, Some(*classifier)),
        _ => {
            return Err(Error::InvalidNotation {
                label: invalid_label.to_owned(),
                notation: notation.to_owned(),
            });
        }
    };

    let group_path = group.replace('.', "/");
    let file_name = match classifier {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{extension}"),
        None => format!("{artifact}-{version}.{extension}"),
    };

    Ok(format!("{group_path}/{artifact}/{version}/{file_name}"))
}

pub fn classifier_notation(notation: &str, classifier: &str) -> String {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    format!("{coordinates}:{classifier}@{extension}")
}

pub async fn fetch_maven_metadata(
    client: &reqwest::Client,
    url: String,
    source_label: &str,
) -> Result<MavenMetadataBody> {
    let raw = fetch_text(client, url.as_str(), source_label).await?;
    from_str(&raw).map_err(|source| Error::DecodeMavenMetadata {
        source_label: source_label.to_owned(),
        url,
        source,
    })
}

pub fn infer_neoforge_game_version(loader_version: &str) -> Option<MinecraftVersionId> {
    let core_version = loader_version.split('-').next().unwrap_or(loader_version);
    let parts = core_version.split('.').collect::<Vec<&str>>();
    let [major, minor, ..] = parts.as_slice() else {
        return None;
    };

    let major_number = major.parse::<u32>().ok()?;
    if major_number >= 26 {
        let game_version = match parts.get(2).copied() {
            Some("0") | None => format!("{major}.{minor}"),
            Some(hotfix) => format!("{major}.{minor}.{hotfix}"),
        };
        return Some(MinecraftVersionId::from(game_version));
    }

    Some(MinecraftVersionId::from(format!("1.{major}.{minor}")))
}
