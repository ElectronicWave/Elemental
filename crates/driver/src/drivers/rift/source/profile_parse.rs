use std::collections::HashMap;

use elemental_schema::{
    fabric::{
        ProfileJson, ProfileLibrary, ProfileLibraryDownloads, ProfileLibraryExtract, ProfileLogging,
    },
    mojang::piston::PistonMetaArguments,
};
use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
struct RiftRawProfileJson {
    id: String,
    #[serde(rename = "inheritsFrom")]
    inherits_from: String,
    arguments: Option<PistonMetaArguments>,
    assets: Option<String>,
    libraries: Vec<RiftRawProfileLibrary>,
    logging: Option<ProfileLogging>,
    #[serde(rename = "mainClass")]
    main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    minimum_launcher_version: Option<usize>,
    #[serde(rename = "type")]
    release_type: String,
    time: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

#[derive(Debug, Deserialize)]
struct RiftRawProfileLibrary {
    name: String,
    #[serde(default)]
    url: String,
    downloads: Option<ProfileLibraryDownloads>,
    natives: Option<HashMap<String, String>>,
    extract: Option<ProfileLibraryExtract>,
}

pub(super) fn parse_profile_json(raw_profile: &str) -> Result<ProfileJson> {
    let raw_profile: RiftRawProfileJson =
        serde_json::from_str(raw_profile).map_err(|source| Error::DecodeRiftProfile { source })?;

    Ok(ProfileJson {
        id: raw_profile.id,
        inherits_from: raw_profile.inherits_from,
        arguments: raw_profile.arguments,
        assets: raw_profile.assets,
        libraries: raw_profile
            .libraries
            .into_iter()
            .map(raw_profile_library_to_profile)
            .collect(),
        logging: raw_profile.logging,
        main_class: raw_profile.main_class,
        minimum_launcher_version: raw_profile.minimum_launcher_version,
        release_type: raw_profile.release_type,
        time: raw_profile.time,
        release_time: raw_profile.release_time,
    })
}

fn raw_profile_library_to_profile(raw_library: RiftRawProfileLibrary) -> ProfileLibrary {
    ProfileLibrary {
        name: raw_library.name.clone(),
        // Legacy Rift profiles omit the launcher libraries base for Mojang-hosted artifacts.
        url: ensure_trailing_slash(raw_library.url.as_str()),
        downloads: raw_library.downloads,
        natives: raw_library.natives,
        extract: raw_library.extract,
    }
}

fn ensure_trailing_slash(url: &str) -> String {
    if url.is_empty() || url.ends_with('/') {
        return url.to_owned();
    }

    format!("{url}/")
}
