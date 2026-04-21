use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use elemental_core::{runtime::distribution::Distribution, storage::Storage};
use elemental_infra::{
    downloader::{
        core::ElementalDownloader,
        task::{DownloadPlan, DownloadTask},
    },
    jar::JarFile,
};
use regex::Regex;
use tokio::{fs::create_dir_all, process::Command};

use crate::{
    drivers::neoforge::prepared::NeoForgeRemoteResolver,
    families::{
        installer::{InstallerArchive, InstallerArtifact, installer_coordinate_path},
        version_json::{
            VersionJsonGameStorageExt, VersionJsonInstanceLayout, VersionJsonRootLayout,
            VersionJsonVersionStorageExt, classpath::join_classpath,
        },
    },
};

use elemental_schema::forge::{ForgeInstallerProcessor, ForgeInstallerProfile};

const CLIENT_PROCESSOR_DATA_KEYS: &[&str] = &[
    "MAPPINGS",
    "MOJMAPS",
    "MERGED_MAPPINGS",
    "MC_SLIM",
    "MC_EXTRA",
    "MC_SRG",
    "PATCHED",
];

pub async fn ensure_profile_libraries_downloaded<L, VL>(
    downloader: &ElementalDownloader,
    remote_resolver: &NeoForgeRemoteResolver,
    instance: &Storage<VL, Storage<L>>,
    install_profile: &ForgeInstallerProfile,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let mut seen = HashSet::new();
    let mut tasks = Vec::new();

    for library in &install_profile.libraries {
        if let Some(artifact) = &library.downloads.artifact {
            let path = instance.parent.library_path(artifact.path.as_str())?;
            if seen.insert(path.clone()) {
                tasks.push(DownloadTask::new(
                    remote_resolver
                        .neoforge_artifact_url(artifact.url.as_str(), artifact.path.as_str())?,
                    path,
                    artifact.size.map(|size| size as u64),
                    artifact.sha1.clone(),
                ));
            }
        }

        if let Some(classifiers) = &library.downloads.classifiers {
            for artifact in classifiers.values() {
                let path = instance.parent.library_path(artifact.path.as_str())?;
                if seen.insert(path.clone()) {
                    tasks.push(DownloadTask::new(
                        remote_resolver
                            .neoforge_artifact_url(artifact.url.as_str(), artifact.path.as_str())?,
                        path,
                        artifact.size.map(|size| size as u64),
                        artifact.sha1.clone(),
                    ));
                }
            }
        }
    }

    if tasks.is_empty() {
        return Ok(());
    }

    let report = downloader
        .run_plan(DownloadPlan::named(
            "neoforge-install-profile".to_owned(),
            tasks,
        ))
        .await
        .context("download neoforge install profile libraries failed")?;

    if report.failed > 0 {
        let failures = report
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.task_id, failure.error))
            .collect::<Vec<String>>()
            .join("\n");
        bail!("neoforge install profile library download failed:\n{failures}");
    }

    Ok(())
}

pub async fn run_client_processors<L, VL>(
    runtime: &Distribution,
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
) -> Result<()>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    let archive = InstallerArchive::new(installer_artifact.path.clone());
    let client_processors = install_profile
        .processors
        .iter()
        .filter(|processor| processor_applies_to_client(processor))
        .collect::<Vec<&ForgeInstallerProcessor>>();
    if client_processors.is_empty() {
        return Ok(());
    }

    let context = NeoForgeProcessorContext::new(
        absolute_path(&instance.parent.path)?,
        absolute_path(&instance.path)?
            .join(".elemental")
            .join("neoforge"),
        absolute_path(&instance.parent.path.join("libraries"))?,
        absolute_path(&instance.jar_path()?)?,
        installer_artifact,
        install_profile,
        archive,
    );

    if context.client_outputs_ready()? {
        return Ok(());
    }

    context.ensure_client_directories().await?;

    for processor in client_processors {
        run_processor(runtime, &context, processor).await?;
    }

    if !context.client_outputs_ready()? {
        bail!(
            "neoforge client processors completed without producing the expected client artifacts"
        );
    }

    Ok(())
}

pub fn client_processors_ready<L, VL>(
    instance: &Storage<VL, Storage<L>>,
    installer_artifact: &InstallerArtifact,
    install_profile: &ForgeInstallerProfile,
) -> Result<bool>
where
    L: VersionJsonRootLayout,
    VL: VersionJsonInstanceLayout,
{
    if install_profile
        .processors
        .iter()
        .all(|processor| !processor_applies_to_client(processor))
    {
        return Ok(true);
    }

    let archive = InstallerArchive::new(installer_artifact.path.clone());
    let context = NeoForgeProcessorContext::new(
        absolute_path(&instance.parent.path)?,
        absolute_path(&instance.path)?
            .join(".elemental")
            .join("neoforge"),
        absolute_path(&instance.parent.path.join("libraries"))?,
        absolute_path(&instance.jar_path()?)?,
        installer_artifact,
        install_profile,
        archive,
    );
    context.client_outputs_ready()
}

fn processor_applies_to_client(processor: &ForgeInstallerProcessor) -> bool {
    processor.sides.is_empty() || processor.sides.iter().any(|side| side == "client")
}

#[derive(Clone)]
struct NeoForgeProcessorContext<'a> {
    installer_artifact: &'a InstallerArtifact,
    install_profile: &'a ForgeInstallerProfile,
    archive: InstallerArchive<PathBuf>,
    root_directory: PathBuf,
    libraries_directory: PathBuf,
    neoforge_state_directory: PathBuf,
    minecraft_jar_path: PathBuf,
}

impl<'a> NeoForgeProcessorContext<'a> {
    fn new(
        root_directory: PathBuf,
        neoforge_state_directory: PathBuf,
        libraries_directory: PathBuf,
        minecraft_jar_path: PathBuf,
        installer_artifact: &'a InstallerArtifact,
        install_profile: &'a ForgeInstallerProfile,
        archive: InstallerArchive<PathBuf>,
    ) -> Self {
        Self {
            installer_artifact,
            install_profile,
            archive,
            root_directory,
            libraries_directory,
            neoforge_state_directory,
            minecraft_jar_path,
        }
    }

    async fn ensure_client_directories(&self) -> Result<()> {
        for path in self.required_client_output_paths()? {
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await?;
            }
        }

        Ok(())
    }

    fn client_outputs_ready(&self) -> Result<bool> {
        let paths = self.required_client_output_paths()?;
        if paths.is_empty() {
            return Ok(false);
        }

        Ok(paths.iter().all(|path| path.exists()))
    }

    fn required_client_output_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for key in CLIENT_PROCESSOR_DATA_KEYS {
            let Some(value) = self.resolve_data_value(key)? else {
                continue;
            };

            let path = PathBuf::from(value);
            paths.push(path);
        }

        Ok(paths)
    }

    fn resolve_argument(&self, value: &str) -> Result<String> {
        let regex = Regex::new(r"\{([^}]+)\}")?;
        let mut resolved = value.to_owned();

        for capture in regex.captures_iter(value) {
            let Some(key) = capture.get(1).map(|group| group.as_str()) else {
                continue;
            };
            let replacement = self.resolve_variable(key)?;
            resolved = resolved.replace(&format!("{{{key}}}"), replacement.as_str());
        }

        self.materialize_data_value(resolved.as_str())
    }

    fn resolve_variable(&self, key: &str) -> Result<String> {
        match key {
            "INSTALLER" => Ok(absolute_path(&self.installer_artifact.path)?
                .to_string_lossy()
                .to_string()),
            "ROOT" => Ok(self.root_directory.to_string_lossy().to_string()),
            "MINECRAFT_JAR" => Ok(self.minecraft_jar_path.to_string_lossy().to_string()),
            "SIDE" => Ok("client".to_owned()),
            _ => self
                .resolve_data_value(key)?
                .with_context(|| format!("unknown neoforge processor variable: {key}")),
        }
    }

    fn resolve_data_value(&self, key: &str) -> Result<Option<String>> {
        let Some(entry) = self.install_profile.data.get(key) else {
            return Ok(None);
        };
        let Some(raw) = entry.client.as_deref().or(entry.server.as_deref()) else {
            return Ok(None);
        };
        Ok(Some(self.materialize_data_value(raw)?))
    }

    fn materialize_data_value(&self, raw: &str) -> Result<String> {
        if raw.starts_with('[') && raw.ends_with(']') {
            let notation = raw.trim_start_matches('[').trim_end_matches(']');
            let path = self
                .libraries_directory
                .join(installer_coordinate_path(notation)?);
            return Ok(path.to_string_lossy().to_string());
        }

        if raw.starts_with('\'') && raw.ends_with('\'') {
            return Ok(raw.trim_matches('\'').to_owned());
        }

        if raw.starts_with('/') {
            let relative_path = raw.trim_start_matches('/');
            let output_path = self
                .neoforge_state_directory
                .join("installer")
                .join(relative_path);
            if !output_path.exists() {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&output_path, self.archive.read_bytes(raw)?)?;
            }

            return Ok(output_path.to_string_lossy().to_string());
        }

        Ok(raw.to_owned())
    }

    fn processor_jar_path(&self, processor: &ForgeInstallerProcessor) -> Result<PathBuf> {
        Ok(self
            .libraries_directory
            .join(installer_coordinate_path(processor.jar.as_str())?))
    }

    fn processor_classpath(&self, processor: &ForgeInstallerProcessor) -> Result<Vec<PathBuf>> {
        let mut entries = Vec::with_capacity(1 + processor.classpath.len());
        entries.push(self.processor_jar_path(processor)?);

        for coordinate in &processor.classpath {
            entries.push(
                self.libraries_directory
                    .join(installer_coordinate_path(coordinate)?),
            );
        }

        Ok(entries)
    }
}

async fn run_processor(
    runtime: &Distribution,
    context: &NeoForgeProcessorContext<'_>,
    processor: &ForgeInstallerProcessor,
) -> Result<()> {
    let processor_jar_path = context.processor_jar_path(processor)?;
    let main_class = read_jar_main_class(&processor_jar_path)?;
    let classpath_entries = context.processor_classpath(processor)?;
    let classpath = join_classpath(
        classpath_entries
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    );
    let resolved_args = processor
        .args
        .iter()
        .map(|arg| context.resolve_argument(arg.as_str()))
        .collect::<Result<Vec<String>>>()?;

    ensure_parent_directories(&resolved_args).await?;

    let output = Command::new(runtime.executable())
        .arg("-cp")
        .arg(&classpath)
        .arg(main_class.as_str())
        .args(resolved_args.as_slice())
        .current_dir(&context.root_directory)
        .output()
        .await
        .with_context(|| format!("run neoforge processor failed: {}", processor.jar.as_str()))?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!(
        "neoforge processor failed: processor={}, java={}, main_class={}, classpath={}, args={:?}, exit_status={:?}, stdout=\n{}\nstderr=\n{}",
        processor.jar,
        runtime.executable().display(),
        main_class,
        classpath,
        resolved_args,
        output.status.code(),
        stdout,
        stderr
    );
}

fn read_jar_main_class(path: &Path) -> Result<String> {
    let manifest = JarFile::new(path).by_name_string("META-INF/MANIFEST.MF")?;
    let unfolded = unfold_manifest_lines(manifest.as_str());
    let main_class = unfolded
        .lines()
        .find_map(|line| line.strip_prefix("Main-Class: "))
        .map(str::trim)
        .map(ToOwned::to_owned)
        .with_context(|| format!("jar manifest is missing Main-Class: {}", path.display()))?;
    Ok(main_class)
}

fn unfold_manifest_lines(manifest: &str) -> String {
    let mut lines: Vec<String> = Vec::new();

    for line in manifest.lines() {
        if let Some(last) = lines.last_mut()
            && let Some(continued) = line.strip_prefix(' ')
        {
            last.push_str(continued);
            continue;
        }

        lines.push(line.to_owned());
    }

    lines.join("\n")
}

async fn ensure_parent_directories(arguments: &[String]) -> Result<()> {
    let mut seen = HashSet::new();

    for argument in arguments {
        let path = PathBuf::from(argument);
        let Some(parent) = path.parent() else {
            continue;
        };
        if parent.as_os_str().is_empty() {
            continue;
        }
        if !seen.insert(parent.to_path_buf()) {
            continue;
        }

        create_dir_all(parent).await?;
    }

    Ok(())
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let current_directory = std::env::current_dir()
        .with_context(|| format!("resolve current directory failed for {}", path.display()))?;
    Ok(current_directory.join(path))
}
