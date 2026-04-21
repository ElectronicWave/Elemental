use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_infra::downloader::{core::ElementalDownloader, task::DownloadPlan};
use elemental_schema::{
    forge::ForgeInstallerProfile,
    mojang::piston::{
        PistonMetaArguments, PistonMetaAssetIndex, PistonMetaDownloads, PistonMetaJavaVersion,
        PistonMetaLibraries, PistonMetaLogging,
    },
};
use serde::Deserialize;
use tokio::fs::create_dir_all;

use crate::{
    drivers::{
        forge::{
            processor::{
                client_processors_ready, ensure_profile_libraries_downloaded, run_client_processors,
            },
            source::{ForgeEndpoints, ForgeSource, parse_installer_version},
        },
        vanilla::source::{VanillaEndpoints, VanillaSource},
    },
    families::{
        installer::{InstallerArchive, InstallerArtifact},
        version_json::{
            PreparedVersionJsonInstance, ResolvedVersionJsonInstance, ResolvedVersionJsonMetadata,
            VersionJsonGameStorageExt, VersionJsonInstanceLayout, VersionJsonRemoteResolver,
            VersionJsonRootLayout,
        },
    },
};

const INSTALL_PROFILE_ENTRY: &str = "install_profile.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeInstallStatus {
    pub installer_downloaded: bool,
    pub install_profile_persisted: bool,
    pub embedded_version_persisted: bool,
    pub profile_libraries_ready: bool,
    pub processors_completed: bool,
    pub launch_version_ready: bool,
}

#[derive(Debug, Clone)]
pub struct ForgeRemoteResolver {
    vanilla_endpoints: VanillaEndpoints,
    forge_endpoints: ForgeEndpoints,
}

pub type ResolvedForgeMetadata = ResolvedVersionJsonMetadata<ForgeRemoteResolver>;
pub type ResolvedForgeLaunchVersion<L, VL> =
    ResolvedVersionJsonInstance<ForgeRemoteResolver, L, VL>;
pub type PreparedForgeLaunchVersion<L, VL> =
    PreparedVersionJsonInstance<ForgeRemoteResolver, L, VL>;

#[derive(Debug, Clone)]
pub struct ResolvedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub source: ForgeSource,
    pub instance: Storage<VL, Storage<L>>,
    pub game_version: String,
    pub loader_version: String,
    pub installer_artifact: InstallerArtifact,
}

#[derive(Debug, Clone)]
pub struct PreparedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub resolved_version: ResolvedForgeVersion<L, VL>,
    pub install_profile: ForgeInstallerProfile,
    pub embedded_version: Option<serde_json::Value>,
    pub launch_version: PreparedForgeLaunchVersion<L, VL>,
    pub install_status: ForgeInstallStatus,
}

impl ForgeRemoteResolver {
    pub fn new(vanilla_endpoints: VanillaEndpoints, forge_endpoints: ForgeEndpoints) -> Self {
        Self {
            vanilla_endpoints,
            forge_endpoints,
        }
    }

    pub fn forge_artifact_url(&self, raw_url: &str, artifact_path: &str) -> Result<String> {
        if raw_url.trim().is_empty() {
            return self.forge_endpoints.maven_artifact_url(artifact_path);
        }

        self.forge_endpoints.rewrite_upstream(raw_url)
    }
}

impl VersionJsonRemoteResolver for ForgeRemoteResolver {
    fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        if let Ok(rewritten) = self.vanilla_endpoints.rewrite_upstream(raw_url) {
            return Ok(rewritten);
        }

        self.forge_endpoints
            .rewrite_upstream(raw_url)
            .with_context(|| format!("rewrite forge upstream url failed for '{raw_url}'"))
    }

    fn object_url(&self, hash: &str) -> Result<String> {
        self.vanilla_endpoints.object_url(hash)
    }
}

impl<L, VL> ResolvedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    pub async fn prepare(
        &self,
        downloader: &ElementalDownloader,
        vanilla_source: &VanillaSource,
        remote_resolver: &ForgeRemoteResolver,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        ensure_installer_downloaded(downloader, &self.installer_artifact).await?;

        let archive = InstallerArchive::new(self.installer_artifact.path.clone());
        let install_profile = archive
            .read_json::<ForgeInstallerProfile>(INSTALL_PROFILE_ENTRY)
            .context("read forge install_profile.json from installer failed")?;
        let embedded_version = read_embedded_version(&archive, &install_profile)?;

        persist_install_profile(&self.instance.path, &install_profile).await?;
        persist_embedded_version(&self.instance.path, embedded_version.as_ref()).await?;
        archive
            .extract_maven_artifacts(&self.instance.parent.path.join("libraries"))
            .context("extract forge embedded maven artifacts failed")?;

        let launch_version = prepare_launch_version(
            self,
            vanilla_source,
            remote_resolver,
            downloader,
            embedded_version.as_ref(),
        )
        .await?;
        ensure_profile_libraries_downloaded(
            downloader,
            remote_resolver,
            &self.instance,
            &install_profile,
        )
        .await?;

        let runtime = processor_runtime(&launch_version).await?;
        run_client_processors(
            &runtime,
            &self.instance,
            &self.installer_artifact,
            &install_profile,
        )
        .await?;

        let install_status =
            install_status(self, &install_profile, &launch_version.resolved_version).await?;

        Ok(PreparedForgeVersion {
            resolved_version: self.clone(),
            install_profile,
            embedded_version,
            launch_version,
            install_status,
        })
    }

    pub async fn load(
        source: ForgeSource,
        remote_resolver: ForgeRemoteResolver,
        instance: Storage<VL, Storage<L>>,
    ) -> Result<PreparedForgeVersion<L, VL>> {
        let install_profile_path = install_profile_path(&instance.path);
        let install_profile = serde_json::from_reader(std::fs::File::open(&install_profile_path)?)
            .with_context(|| {
                format!(
                    "read persisted forge install profile failed: {}",
                    install_profile_path.display()
                )
            })?;

        let embedded_version_path = embedded_version_path(&instance.path);
        let embedded_version = if embedded_version_path.exists() {
            Some(
                serde_json::from_reader(std::fs::File::open(&embedded_version_path)?)
                    .with_context(|| {
                        format!(
                            "read persisted forge embedded version failed: {}",
                            embedded_version_path.display()
                        )
                    })?,
            )
        } else {
            None
        };

        let (game_version, loader_version) = profile_identity(&install_profile)?;
        let installer_artifact =
            source.installer_artifact(&instance.parent, &game_version, &loader_version)?;
        let resolved_version = ResolvedForgeVersion {
            source,
            instance: instance.clone(),
            game_version,
            loader_version,
            installer_artifact,
        };
        let launch_version = ResolvedForgeLaunchVersion::load(remote_resolver, instance.clone())?
            .into_prepared()
            .await?;
        let install_status = install_status(
            &resolved_version,
            &install_profile,
            &launch_version.resolved_version,
        )
        .await?;

        if !install_status.processors_completed {
            bail!(
                "local forge instance '{}' is not fully prepared: {:?}",
                instance.name().context("get forge instance name failed")?,
                install_status
            );
        }

        Ok(PreparedForgeVersion {
            resolved_version,
            install_profile,
            embedded_version,
            launch_version,
            install_status,
        })
    }
}

impl<L, VL> PreparedForgeVersion<L, VL>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    pub fn install_profile_path(&self) -> PathBuf {
        install_profile_path(&self.resolved_version.instance.path)
    }

    pub fn embedded_version_path(&self) -> PathBuf {
        embedded_version_path(&self.resolved_version.instance.path)
    }

    pub fn required_java_major_version(&self) -> usize {
        self.launch_version.required_java_major_version()
    }
}

fn read_embedded_version<P: AsRef<Path>>(
    archive: &InstallerArchive<P>,
    install_profile: &ForgeInstallerProfile,
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
            .with_context(|| format!("read forge embedded version json failed: {json_path}"))?,
    ))
}

async fn ensure_installer_downloaded(
    downloader: &ElementalDownloader,
    installer_artifact: &InstallerArtifact,
) -> Result<()> {
    let report = downloader
        .run_plan(DownloadPlan::named(
            format!("forge-installer-{}", installer_artifact.coordinate),
            vec![installer_artifact.download_task()],
        ))
        .await
        .context("download forge installer failed")?;

    if report.failed > 0 {
        let failures = report
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.task_id, failure.error))
            .collect::<Vec<String>>()
            .join("\n");
        bail!("forge installer download failed:\n{failures}");
    }

    Ok(())
}

async fn prepare_launch_version<L, VL>(
    resolved_version: &ResolvedForgeVersion<L, VL>,
    vanilla_source: &VanillaSource,
    remote_resolver: &ForgeRemoteResolver,
    downloader: &ElementalDownloader,
    embedded_version: Option<&serde_json::Value>,
) -> Result<PreparedForgeLaunchVersion<L, VL>>
where
    L: VersionJsonRootLayout + Clone,
    VL: VersionJsonInstanceLayout + Clone,
{
    let embedded_version = embedded_version
        .context("forge installer is missing an embedded version json; launchable forge preparation is not available")?;
    let base_metadata =
        resolve_vanilla_metadata(vanilla_source, &resolved_version.game_version).await?;
    let merged_metadata = merge_embedded_version(
        base_metadata.metadata,
        embedded_version,
        resolved_version.source.endpoints(),
    )?;
    let launch_version = ResolvedForgeMetadata::new(
        remote_resolver.clone(),
        merged_metadata,
        base_metadata.asset_index_objects,
    )
    .persist(&resolved_version.instance)
    .await?;

    launch_version.prepare(downloader).await
}

async fn processor_runtime<L, VL>(
    launch_version: &PreparedForgeLaunchVersion<L, VL>,
) -> Result<Distribution>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let required_major_version = launch_version.required_java_major_version();
    Distribution::find_cached_by_java_major(required_major_version)
        .await
        .with_context(|| {
            format!(
                "can't find a local Java runtime with major version {} for forge processors",
                required_major_version
            )
        })
}

async fn resolve_vanilla_metadata(
    vanilla_source: &VanillaSource,
    game_version: &str,
) -> Result<crate::drivers::vanilla::prepared::ResolvedVanillaMetadata> {
    let launchmeta = vanilla_source.launch_meta().await?;
    let metadata_url = launchmeta
        .versions
        .iter()
        .find(|version| version.id == game_version)
        .with_context(|| format!("can't find vanilla version named '{game_version}'"))?
        .url
        .clone();
    let metadata = vanilla_source.piston_meta(metadata_url).await?;
    let asset_index_objects = vanilla_source
        .asset_index_objects(&metadata.asset_index.url)
        .await?;

    Ok(
        crate::drivers::vanilla::prepared::ResolvedVanillaMetadata::new(
            vanilla_source.endpoints().clone(),
            metadata,
            asset_index_objects,
        ),
    )
}

async fn install_status<L, VL>(
    resolved_version: &ResolvedForgeVersion<L, VL>,
    install_profile: &ForgeInstallerProfile,
    launch_version: &ResolvedForgeLaunchVersion<L, VL>,
) -> Result<ForgeInstallStatus>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    Ok(ForgeInstallStatus {
        installer_downloaded: resolved_version.installer_artifact.path.exists(),
        install_profile_persisted: install_profile_path(&resolved_version.instance.path).exists(),
        embedded_version_persisted: embedded_version_path(&resolved_version.instance.path).exists(),
        profile_libraries_ready: profile_libraries_ready(
            &resolved_version.instance,
            install_profile,
        )?,
        processors_completed: client_processors_ready(
            &resolved_version.instance,
            &resolved_version.installer_artifact,
            install_profile,
        )?,
        launch_version_ready: launch_version.status().await?.is_ready(),
    })
}

fn profile_libraries_ready<L, VL>(
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
                .library_path(artifact.path.as_str())?
                .exists()
        {
            return Ok(false);
        }

        if let Some(classifiers) = &library.downloads.classifiers {
            for artifact in classifiers.values() {
                if !instance
                    .parent
                    .library_path(artifact.path.as_str())?
                    .exists()
                {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

async fn persist_install_profile(
    instance_root: &Path,
    install_profile: &ForgeInstallerProfile,
) -> Result<()> {
    let path = install_profile_path(instance_root);
    let parent = path
        .parent()
        .context("forge install profile path has no parent directory")?;
    create_dir_all(parent).await?;
    tokio::fs::write(path, serde_json::to_vec_pretty(install_profile)?).await?;
    Ok(())
}

async fn persist_embedded_version(
    instance_root: &Path,
    embedded_version: Option<&serde_json::Value>,
) -> Result<()> {
    let path = embedded_version_path(instance_root);
    if let Some(version) = embedded_version {
        let parent = path
            .parent()
            .context("forge embedded version path has no parent directory")?;
        create_dir_all(parent).await?;
        tokio::fs::write(path, serde_json::to_vec_pretty(version)?).await?;
        return Ok(());
    }

    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

fn forge_state_root(instance_root: &Path) -> PathBuf {
    instance_root.join(".elemental").join("forge")
}

fn install_profile_path(instance_root: &Path) -> PathBuf {
    forge_state_root(instance_root).join("install_profile.json")
}

fn embedded_version_path(instance_root: &Path) -> PathBuf {
    forge_state_root(instance_root).join("version.json")
}

fn profile_identity(install_profile: &ForgeInstallerProfile) -> Result<(String, String)> {
    let game_version = install_profile
        .minecraft
        .clone()
        .or_else(|| {
            install_profile
                .install
                .as_ref()
                .and_then(|legacy| legacy.minecraft.clone())
        })
        .context("forge install profile is missing minecraft version")?;

    let raw_version = install_profile
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
        .context("forge install profile is missing forge version identity")?;

    let (_, loader_version) = parse_installer_version(&raw_version)?;
    Ok((game_version, loader_version))
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

fn merge_embedded_version(
    base_metadata: elemental_schema::mojang::piston::PistonMetaData,
    embedded_version: &serde_json::Value,
    forge_endpoints: &ForgeEndpoints,
) -> Result<elemental_schema::mojang::piston::PistonMetaData> {
    let mut embedded = serde_json::from_value::<EmbeddedVersionData>(embedded_version.clone())
        .context("decode forge embedded version json failed")?;
    embedded.libraries = normalize_forge_library_urls(embedded.libraries, forge_endpoints)?;
    let (merged_arguments, merged_minecraft_arguments) = merge_arguments(
        base_metadata.arguments.clone(),
        base_metadata.minecraft_arguments.clone(),
        embedded.arguments,
        embedded.minecraft_arguments,
    )?;

    Ok(elemental_schema::mojang::piston::PistonMetaData {
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
            game: crate::launch_arguments::parse_argument_string(arguments.as_str())?
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

fn normalize_forge_library_urls(
    libraries: Vec<PistonMetaLibraries>,
    forge_endpoints: &ForgeEndpoints,
) -> Result<Vec<PistonMetaLibraries>> {
    let mut normalized = Vec::with_capacity(libraries.len());

    for mut library in libraries {
        if let Some(artifact) = &mut library.downloads.artifact
            && artifact.url.trim().is_empty()
        {
            artifact.url = forge_endpoints.maven_artifact_url(artifact.path.as_str())?;
        }

        if let Some(classifiers) = &mut library.downloads.classifiers {
            for artifact in classifiers.values_mut() {
                if artifact.url.trim().is_empty() {
                    artifact.url = forge_endpoints.maven_artifact_url(artifact.path.as_str())?;
                }
            }
        }

        normalized.push(library);
    }

    Ok(normalized)
}

fn merge_libraries(
    base_libraries: Vec<PistonMetaLibraries>,
    embedded_libraries: Vec<PistonMetaLibraries>,
) -> Vec<PistonMetaLibraries> {
    let mut seen = base_libraries
        .iter()
        .map(|library| library.name.clone())
        .collect::<std::collections::HashSet<String>>();
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
