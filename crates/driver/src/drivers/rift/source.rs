use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use elemental_infra::jar::JarBytes;
use elemental_schema::{
    fabric::{
        ProfileJson, ProfileLibrary, ProfileLibraryArtifact, ProfileLibraryDownloads,
        ProfileLibraryExtract, ProfileLogging,
    },
    mojang::piston::PistonMetaArguments,
};
use serde::Deserialize;

use crate::{
    http::{build_default_client, fetch_bytes, fetch_json},
    url::{Origin, OriginPolicy},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiftOrigin {
    GitHubApi,
    GitHubReleases,
    SpongeMaven,
    MavenCentral,
}

#[derive(Debug, Clone)]
pub struct RiftEndpoints {
    origin_policy: OriginPolicy<RiftOrigin>,
}

#[derive(Debug, Clone)]
pub struct RiftSource {
    client: reqwest::Client,
    endpoints: RiftEndpoints,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiftRelease {
    pub tag_name: String,
    pub loader_version: String,
    pub asset_name: String,
    pub asset_size: Option<usize>,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    draft: bool,
    published_at: Option<String>,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    size: usize,
}

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

impl Default for RiftEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for RiftOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::GitHubApi => "https://api.github.com",
            Self::GitHubReleases => "https://github.com",
            Self::SpongeMaven => "https://repo.spongepowered.org/maven",
            Self::MavenCentral => "https://repo.maven.apache.org/maven2",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[RiftOrigin] = &[
            RiftOrigin::GitHubApi,
            RiftOrigin::GitHubReleases,
            RiftOrigin::SpongeMaven,
            RiftOrigin::MavenCentral,
        ];
        ALL
    }
}

impl RiftEndpoints {
    pub fn new(origin_policy: OriginPolicy<RiftOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn releases_url(&self) -> Result<String> {
        self.origin_policy.resolve_segments(
            RiftOrigin::GitHubApi,
            ["repos", "DimensionalDevelopment", "Rift", "releases"],
        )
    }

    pub fn release_asset_url(&self, tag_name: &str, asset_name: &str) -> Result<String> {
        self.origin_policy.resolve_segments(
            RiftOrigin::GitHubReleases,
            [
                "DimensionalDevelopment",
                "Rift",
                "releases",
                "download",
                tag_name,
                asset_name,
            ],
        )
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.origin_policy.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }

    pub fn sponge_maven_url(&self) -> Result<String> {
        self.origin_policy.resolve(RiftOrigin::SpongeMaven, "")
    }

    pub fn maven_central_url(&self) -> Result<String> {
        self.origin_policy.resolve(RiftOrigin::MavenCentral, "")
    }
}

impl Default for RiftSource {
    fn default() -> Self {
        Self {
            client: build_default_client("rift source"),
            endpoints: RiftEndpoints::default(),
        }
    }
}

impl RiftSource {
    pub fn new(endpoints: RiftEndpoints) -> Self {
        Self {
            endpoints,
            ..Self::default()
        }
    }

    pub fn endpoints(&self) -> &RiftEndpoints {
        &self.endpoints
    }

    pub async fn releases(&self) -> Result<Vec<RiftRelease>> {
        let url = self.endpoints.releases_url()?;
        let releases: Vec<GitHubRelease> =
            fetch_json(&self.client, url.as_str(), "rift source").await?;

        releases
            .into_iter()
            .filter(|release| !release.draft)
            .map(build_rift_release)
            .collect()
    }

    pub async fn profile_json(&self, loader_version: &str) -> Result<ProfileJson> {
        let release = release_for_loader_version(loader_version);
        self.profile_json_for_release(&release).await
    }

    pub async fn profile_json_for_release(&self, release: &RiftRelease) -> Result<ProfileJson> {
        let url = self
            .endpoints
            .release_asset_url(release.tag_name.as_str(), release.asset_name.as_str())?;
        let jar_bytes = fetch_bytes(&self.client, url.as_str(), "rift source").await?;
        let raw_profile = read_release_profile_json(release, jar_bytes).await?;
        let profile = parse_profile_json(raw_profile.as_str())?;

        normalize_profile_json(profile, release, &self.endpoints)
    }
}

fn build_rift_release(release: GitHubRelease) -> Result<RiftRelease> {
    let loader_version = normalize_loader_version(release.tag_name.as_str())?;
    let expected_asset_name = format!("Rift-{loader_version}.jar");
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == expected_asset_name)
        .with_context(|| format!("can't find Rift release asset named '{expected_asset_name}'"))?;

    Ok(RiftRelease {
        tag_name: release.tag_name,
        loader_version,
        asset_name: asset.name,
        asset_size: Some(asset.size),
        published_at: release.published_at,
    })
}

fn release_for_loader_version(loader_version: &str) -> RiftRelease {
    RiftRelease {
        tag_name: format!("v{loader_version}"),
        loader_version: loader_version.to_owned(),
        asset_name: format!("Rift-{loader_version}.jar"),
        asset_size: None,
        published_at: None,
    }
}

fn normalize_loader_version(tag_name: &str) -> Result<String> {
    let loader_version = tag_name.trim_start_matches('v').trim();
    if loader_version.is_empty() {
        bail!("Rift release tag has no loader version: '{tag_name}'");
    }

    Ok(loader_version.to_owned())
}

async fn read_release_profile_json(release: &RiftRelease, jar_bytes: Vec<u8>) -> Result<String> {
    JarBytes::new(jar_bytes.as_slice())
        .by_name_string("profile.json")
        .with_context(|| {
            format!(
                "read embedded Rift profile failed from '{}'",
                release.asset_name
            )
        })
}

fn parse_profile_json(raw_profile: &str) -> Result<ProfileJson> {
    let raw_profile: RiftRawProfileJson =
        serde_json::from_str(raw_profile).context("decode embedded Rift profile failed")?;

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

fn normalize_profile_json(
    mut profile: ProfileJson,
    release: &RiftRelease,
    endpoints: &RiftEndpoints,
) -> Result<ProfileJson> {
    profile.libraries = profile
        .libraries
        .into_iter()
        .map(|library| normalize_profile_library(library, endpoints))
        .collect();
    let asset_url =
        endpoints.release_asset_url(release.tag_name.as_str(), release.asset_name.as_str())?;
    let normalized_loader_library = build_loader_library(release, asset_url);
    // Rift 1.13.2 embeds a transient JitPack coordinate, so pin the selected release jar directly.
    if let Some(index) = profile
        .libraries
        .iter()
        .position(|library| is_rift_loader_library(library.name.as_str()))
    {
        profile.libraries[index] = normalized_loader_library;
    } else {
        profile.libraries.insert(0, normalized_loader_library);
    }
    profile.id = build_profile_id(
        profile.inherits_from.as_str(),
        release.loader_version.as_str(),
    );

    Ok(profile)
}

fn normalize_profile_library(library: ProfileLibrary, endpoints: &RiftEndpoints) -> ProfileLibrary {
    if library.name == "org.dimdev:mixin:0.7.11-SNAPSHOT" {
        // The historical DimDev Maven endpoint is no longer a reliable source for this snapshot.
        return build_rift_mixin_library(endpoints);
    }

    if library.url == "http://repo1.maven.org/maven2/"
        && let Ok(maven_central_url) = endpoints.maven_central_url()
    {
        return ProfileLibrary {
            url: ensure_trailing_slash(maven_central_url.as_str()),
            ..library
        };
    }

    if library.url.is_empty() && library.name.starts_with("net.minecraft:") {
        return ProfileLibrary {
            url: "https://libraries.minecraft.net/".to_owned(),
            ..library
        };
    }

    library
}

fn build_loader_library(release: &RiftRelease, asset_url: String) -> ProfileLibrary {
    let library_name = format!("org.dimdev:rift:{}", release.loader_version);
    let artifact_path = format!(
        "org/dimdev/rift/{}/{}-{}.jar",
        release.loader_version, "rift", release.loader_version
    );

    ProfileLibrary {
        name: library_name,
        url: asset_url.clone(),
        downloads: Some(ProfileLibraryDownloads {
            artifact: Some(ProfileLibraryArtifact {
                path: artifact_path,
                url: asset_url,
                size: release.asset_size,
                sha1: None,
            }),
            classifiers: None,
        }),
        natives: None,
        extract: None,
    }
}

fn build_rift_mixin_library(endpoints: &RiftEndpoints) -> ProfileLibrary {
    let artifact_path = format!(
        "org/spongepowered/mixin/0.7.11-SNAPSHOT/mixin-{}.jar",
        "0.7.11-20180703.121122-1"
    );
    let sponge_maven_url = endpoints
        .sponge_maven_url()
        .unwrap_or_else(|_| "https://repo.spongepowered.org/maven".to_owned());
    let artifact_url = format!("{}/{artifact_path}", sponge_maven_url.trim_end_matches('/'));

    ProfileLibrary {
        name: "org.spongepowered:mixin:0.7.11-SNAPSHOT".to_owned(),
        url: ensure_trailing_slash(sponge_maven_url.as_str()),
        downloads: Some(ProfileLibraryDownloads {
            artifact: Some(ProfileLibraryArtifact {
                path: artifact_path,
                url: artifact_url,
                size: None,
                sha1: None,
            }),
            classifiers: None,
        }),
        natives: None,
        extract: None,
    }
}

fn is_rift_loader_library(library_name: &str) -> bool {
    library_name.starts_with("org.dimdev:rift:")
        || library_name.starts_with("com.github.Chocohead:Rift:")
}

fn build_profile_id(game_version: &str, loader_version: &str) -> String {
    format!("{game_version}-rift-{loader_version}")
}
