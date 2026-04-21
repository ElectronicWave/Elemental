use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use elemental_core::{
    runtime::distribution::Distribution,
    runtime::resolve_runtime,
    storage::{Storage, layout::Layoutable},
};
use elemental_infra::downloader::{core::ElementalDownloader, task::DownloadPlan};
use elemental_schema::{
    forge::ForgeInstallerProfile,
    mojang::piston::{
        PistonMetaArguments, PistonMetaAssetIndex, PistonMetaData, PistonMetaDownloads,
        PistonMetaJavaVersion, PistonMetaLibraries, PistonMetaLogging,
    },
};
use serde::Deserialize;
use tokio::fs::create_dir_all;

use crate::{
    drivers::vanilla::source::{VanillaSource, resolve_vanilla_metadata},
    families::{
        installer::{InstallerArchive, InstallerArtifact, installer_client_processors_ready},
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VersionJsonInstanceLayout, VersionJsonRemoteResolver, VersionJsonRootLayout,
            VersionJsonRootResource,
        },
    },
};

const INSTALL_PROFILE_ENTRY: &str = "install_profile.json";

#[derive(Debug, Clone)]
pub struct InstallerPersistedState {
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerInstallStatus {
    pub installer_downloaded: bool,
    pub install_profile_persisted: bool,
    pub embedded_version_persisted: bool,
    pub profile_libraries_ready: bool,
    pub processors_completed: bool,
    pub launch_version_ready: bool,
}

pub fn read_embedded_version<P: AsRef<Path>>(
    archive: &InstallerArchive<P>,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<Option<serde_json::Value>> {
    if let Some(version_info) = &install_profile.version_info {
        return Ok(Some(version_info.clone()));
    }

    let Some(json_path) = install_profile.json.as_deref() else {
        return Ok(None);
    };

    Ok(Some(
        archive
            .read_json::<serde_json::Value>(json_path)
            .with_context(|| {
                format!("read {family_name} embedded version json failed: {json_path}")
            })?,
    ))
}

pub async fn ensure_installer_artifact_downloaded(
    downloader: &ElementalDownloader,
    installer_artifact: &InstallerArtifact,
    family_name: &str,
) -> Result<()> {
    let report = downloader
        .run_plan(DownloadPlan::named(
            format!("{family_name}-installer-{}", installer_artifact.coordinate),
            vec![installer_artifact.download_task()],
        ))
        .await
        .with_context(|| format!("download {family_name} installer failed"))?;

    if report.failed > 0 {
        let failures = report
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.task_id, failure.error))
            .collect::<Vec<String>>()
            .join("\n");
        bail!("{family_name} installer download failed:\n{failures}");
    }

    Ok(())
}

pub async fn prepare_installer_state<V>(
    downloader: &ElementalDownloader,
    installer_artifact: &InstallerArtifact,
    instance_root: &Path,
    libraries_root: &Path,
    family_name: &str,
    validate_profile: V,
) -> Result<InstallerPersistedState>
where
    V: Fn(&ForgeInstallerProfile) -> Result<()>,
{
    ensure_installer_artifact_downloaded(downloader, installer_artifact, family_name).await?;

    let archive = InstallerArchive::new(installer_artifact.path.clone());
    let install_profile = archive
        .read_json::<ForgeInstallerProfile>(INSTALL_PROFILE_ENTRY)
        .with_context(|| {
            format!("read {family_name} {INSTALL_PROFILE_ENTRY} from installer failed")
        })?;
    validate_profile(&install_profile)?;
    let embedded_version = read_embedded_version(&archive, &install_profile, family_name)?;

    persist_install_profile(instance_root, &install_profile, family_name).await?;
    persist_embedded_version(instance_root, embedded_version.as_ref(), family_name).await?;
    archive
        .extract_maven_artifacts(libraries_root)
        .with_context(|| format!("extract {family_name} embedded maven artifacts failed"))?;

    Ok(InstallerPersistedState {
        install_profile,
        embedded_version,
    })
}

pub async fn prepare_installer_launch_version<RR, L, VL, F>(
    request: InstallerLaunchVersionRequest<'_, RR, L, VL, F>,
) -> Result<PreparedVersionJsonInstance<RR, L, VL>>
where
    RR: VersionJsonRemoteResolver + Clone,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
{
    let embedded_version = request.embedded_version.context(format!(
        "{} installer is missing an embedded version json; launchable {} preparation is not available",
        request.family_name, request.family_name
    ))?;
    let base_metadata =
        resolve_vanilla_metadata(request.vanilla_source, request.game_version).await?;
    let merged_metadata = merge_embedded_version(
        base_metadata.metadata,
        embedded_version,
        request.normalize_libraries,
        request.family_name,
    )?;
    let launch_version = ResolvedVersionJsonMetadata::new(
        request.remote_resolver.clone(),
        merged_metadata,
        base_metadata.asset_index_objects,
    )
    .persist(request.instance)
    .await?;

    launch_version.prepare(request.downloader).await
}

pub struct InstallerLaunchVersionRequest<'a, RR, L, VL, F>
where
    RR: VersionJsonRemoteResolver + Clone,
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
{
    pub instance: &'a Storage<VL, Storage<L>>,
    pub game_version: &'a str,
    pub remote_resolver: &'a RR,
    pub downloader: &'a ElementalDownloader,
    pub vanilla_source: &'a VanillaSource,
    pub embedded_version: Option<&'a serde_json::Value>,
    pub normalize_libraries: F,
    pub family_name: &'a str,
}

pub fn load_persisted_installer_state(
    instance_root: &Path,
    family_name: &str,
) -> Result<InstallerPersistedState> {
    let install_profile_path = install_profile_path(instance_root, family_name);
    let install_profile = serde_json::from_reader(std::fs::File::open(&install_profile_path)?)
        .with_context(|| {
            format!(
                "read persisted {family_name} install profile failed: {}",
                install_profile_path.display()
            )
        })?;

    let embedded_version_path = embedded_version_path(instance_root, family_name);
    let embedded_version = if embedded_version_path.exists() {
        Some(
            serde_json::from_reader(std::fs::File::open(&embedded_version_path)?).with_context(
                || {
                    format!(
                        "read persisted {family_name} embedded version failed: {}",
                        embedded_version_path.display()
                    )
                },
            )?,
        )
    } else {
        None
    };

    Ok(InstallerPersistedState {
        install_profile,
        embedded_version,
    })
}

pub fn profile_game_and_raw_loader_version(
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
    loader_name: &str,
) -> Result<(String, String)> {
    let game_version = install_profile
        .minecraft
        .clone()
        .or_else(|| {
            install_profile
                .install
                .as_ref()
                .and_then(|legacy| legacy.minecraft.clone())
        })
        .with_context(|| format!("{family_name} install profile is missing minecraft version"))?;

    let raw_loader_version = install_profile
        .path
        .clone()
        .or_else(|| {
            install_profile
                .install
                .as_ref()
                .and_then(|legacy| legacy.path.clone())
        })
        .and_then(|coordinate| coordinate.split(':').nth(2).map(ToOwned::to_owned))
        .or_else(|| install_profile.version.clone())
        .with_context(|| {
            format!("{family_name} install profile is missing {loader_name} version identity")
        })?;

    Ok((game_version, raw_loader_version))
}

pub fn validate_installer_profile_identity(
    expected_game_version: &str,
    expected_loader_version: &str,
    actual_game_version: &str,
    actual_loader_version: &str,
    family_name: &str,
) -> Result<()> {
    if actual_game_version != expected_game_version {
        bail!(
            "{family_name} installer game version '{}' does not match requested game version '{}'",
            actual_game_version,
            expected_game_version
        );
    }

    if actual_loader_version != expected_loader_version {
        bail!(
            "{family_name} installer loader version '{}' does not match requested loader version '{}'",
            actual_loader_version,
            expected_loader_version
        );
    }

    Ok(())
}

pub fn profile_libraries_ready<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    for library in &install_profile.libraries {
        if let Some(artifact) = &library.downloads.artifact
            && !instance
                .parent
                .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                    artifact.path.as_str(),
                ))))?
                .exists()
        {
            return Ok(false);
        }

        if let Some(classifiers) = &library.downloads.classifiers {
            for artifact in classifiers.values() {
                if !instance
                    .parent
                    .try_get_resource(VersionJsonRootResource::Libraries(Some(PathBuf::from(
                        artifact.path.as_str(),
                    ))))?
                    .exists()
                {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

pub async fn resolve_installer_processor_runtime<RR, L, VL>(
    launch_version: &PreparedVersionJsonInstance<RR, L, VL>,
    runtime_executable_path: Option<&Path>,
    operation_name: &str,
) -> Result<Distribution>
where
    RR: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version = launch_version.required_java_major_version();
    resolve_runtime(
        required_major_version,
        runtime_executable_path,
        operation_name,
    )
    .await
}

pub async fn installer_install_status<RR, L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
    launch_version: &ResolvedVersionJsonInstance<RR, L, VL>,
    family_name: &str,
) -> Result<InstallerInstallStatus>
where
    RR: VersionJsonRemoteResolver,
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    Ok(InstallerInstallStatus {
        installer_downloaded: installer_artifact.path.exists(),
        install_profile_persisted: install_profile_path(&instance.path, family_name).exists(),
        embedded_version_persisted: embedded_version_path(&instance.path, family_name).exists(),
        profile_libraries_ready: profile_libraries_ready(instance, install_profile)?,
        processors_completed: installer_client_processors_ready(
            instance,
            installer_artifact,
            install_profile,
            family_name,
        )?,
        launch_version_ready: launch_version.status().await?.is_ready(),
    })
}

pub async fn persist_install_profile(
    instance_root: &Path,
    install_profile: &ForgeInstallerProfile,
    family_name: &str,
) -> Result<()> {
    let path = install_profile_path(instance_root, family_name);
    let parent = path
        .parent()
        .with_context(|| format!("{family_name} install profile path has no parent directory"))?;
    create_dir_all(parent).await?;
    tokio::fs::write(path, serde_json::to_vec_pretty(install_profile)?).await?;
    Ok(())
}

pub async fn persist_embedded_version(
    instance_root: &Path,
    embedded_version: Option<&serde_json::Value>,
    family_name: &str,
) -> Result<()> {
    let path = embedded_version_path(instance_root, family_name);
    if let Some(version) = embedded_version {
        let parent = path.parent().with_context(|| {
            format!("{family_name} embedded version path has no parent directory")
        })?;
        create_dir_all(parent).await?;
        tokio::fs::write(path, serde_json::to_vec_pretty(version)?).await?;
        return Ok(());
    }

    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

pub fn install_profile_path(instance_root: &Path, family_name: &str) -> PathBuf {
    installer_state_root(instance_root, family_name).join(INSTALL_PROFILE_ENTRY)
}

pub fn embedded_version_path(instance_root: &Path, family_name: &str) -> PathBuf {
    installer_state_root(instance_root, family_name).join("version.json")
}

fn installer_state_root(instance_root: &Path, family_name: &str) -> PathBuf {
    instance_root.join(".elemental").join(family_name)
}

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

fn merge_embedded_version<F>(
    base_metadata: PistonMetaData,
    embedded_version: &serde_json::Value,
    normalize_libraries: F,
    family_name: &str,
) -> Result<PistonMetaData>
where
    F: Fn(Vec<PistonMetaLibraries>) -> Result<Vec<PistonMetaLibraries>>,
{
    let mut embedded = serde_json::from_value::<EmbeddedVersionData>(embedded_version.clone())
        .with_context(|| format!("decode {family_name} embedded version json failed"))?;
    embedded.libraries = normalize_libraries(embedded.libraries)?;
    let (merged_arguments, merged_minecraft_arguments) = merge_arguments(
        base_metadata.arguments.clone(),
        base_metadata.minecraft_arguments.clone(),
        embedded.arguments,
        embedded.minecraft_arguments,
    )?;

    Ok(PistonMetaData {
        arguments: merged_arguments,
        minecraft_arguments: merged_minecraft_arguments,
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
        logging: merge_logging(base_metadata.logging, embedded.logging),
        main_class: embedded.main_class.unwrap_or(base_metadata.main_class),
        minimum_launcher_version: embedded
            .minimum_launcher_version
            .unwrap_or(base_metadata.minimum_launcher_version),
        release_type: embedded.release_type.unwrap_or(base_metadata.release_type),
        time: embedded.time.unwrap_or(base_metadata.time),
        release_time: embedded.release_time.unwrap_or(base_metadata.release_time),
    })
}

fn merge_arguments(
    base_arguments: Option<PistonMetaArguments>,
    base_minecraft_arguments: Option<String>,
    embedded_arguments: Option<PistonMetaArguments>,
    embedded_minecraft_arguments: Option<String>,
) -> Result<(Option<PistonMetaArguments>, Option<String>)> {
    if let Some(arguments) = embedded_minecraft_arguments {
        return Ok((None, Some(arguments)));
    }

    let base_arguments = match (base_arguments, base_minecraft_arguments) {
        (Some(arguments), _) => Some(arguments),
        (None, Some(arguments)) => Some(PistonMetaArguments {
            game: crate::families::version_json::parse_argument_string(arguments.as_str())?
                .into_iter()
                .map(elemental_schema::mojang::piston::PistonMetaGenericArgument::Plain)
                .collect(),
            jvm: Vec::new(),
        }),
        (None, None) => None,
    };

    match (base_arguments, embedded_arguments) {
        (None, None) => Ok((None, None)),
        (Some(arguments), None) => Ok((Some(arguments), None)),
        (None, Some(arguments)) => Ok((Some(arguments), None)),
        (Some(mut base), Some(embedded)) => {
            base.game.extend(embedded.game);
            base.jvm.extend(embedded.jvm);
            Ok((Some(base), None))
        }
    }
}

fn merge_libraries(
    base_libraries: Vec<PistonMetaLibraries>,
    embedded_libraries: Vec<PistonMetaLibraries>,
) -> Vec<PistonMetaLibraries> {
    let mut seen = base_libraries
        .iter()
        .map(|library| library.name.clone())
        .collect::<HashSet<String>>();
    let mut merged = base_libraries;

    for library in embedded_libraries {
        if seen.insert(library.name.clone()) {
            merged.push(library);
        }
    }

    merged
}

fn merge_logging(
    base_logging: Option<PistonMetaLogging>,
    embedded_logging: Option<PistonMetaLogging>,
) -> Option<PistonMetaLogging> {
    match (base_logging, embedded_logging) {
        (None, None) => None,
        (Some(base), None) => Some(base),
        (None, Some(embedded)) => Some(embedded),
        (Some(base), Some(embedded)) => Some(PistonMetaLogging {
            client: embedded.client.or(base.client),
        }),
    }
}
