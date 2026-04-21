use anyhow::{Context, Result, bail};
use elemental_schema::forge::MavenMetadataBody;
use quick_xml::de::from_str;

use crate::http::fetch_text;

pub fn artifact_path(notation: &str, invalid_label: &str) -> Result<String> {
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    let segments = coordinates.split(':').collect::<Vec<&str>>();

    let (group, artifact, version, classifier) = match segments.as_slice() {
        [group, artifact, version] => (*group, *artifact, *version, None),
        [group, artifact, version, classifier] => (*group, *artifact, *version, Some(*classifier)),
        _ => bail!("invalid {invalid_label} notation: {notation}"),
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
    from_str(&raw).with_context(|| format!("decode {source_label} maven metadata failed: {url}"))
}
