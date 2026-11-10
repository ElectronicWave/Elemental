use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use elemental_core::runtime::distribution::Distribution;
use elemental_infra::jar::JarFile;
use elemental_schema::forge::ForgeInstallerProcessor;
use tokio::{fs::create_dir_all, process::Command};

use crate::{
    error::{Error, ProcessorFailure, Result},
    families::version_json::classpath::join_classpath,
};

use super::context::InstallerProcessorContext;

pub(super) async fn run_processor(
    runtime: &Distribution,
    context: &InstallerProcessorContext<'_>,
    processor: &ForgeInstallerProcessor,
) -> Result<()> {
    let processor_jar_path = context.processor_jar_path(processor)?;
    let main_class =
        tokio::task::spawn_blocking(move || read_jar_main_class(&processor_jar_path)).await??;
    let classpath = join_processor_classpath(context, processor)?;
    let mut resolved_args = Vec::with_capacity(processor.args.len());
    for argument in &processor.args {
        resolved_args.push(context.resolve_argument(argument.as_str()).await?);
    }

    ensure_output_directories(context, processor).await?;

    let output = Command::new(runtime.executable())
        .arg("-cp")
        .arg(&classpath)
        .arg(main_class.as_str())
        .args(resolved_args.as_slice())
        .current_dir(&context.root_directory)
        .output()
        .await
        .map_err(|source| Error::RunProcessor {
            family_name: context.family_name.to_owned(),
            processor: processor.jar.clone(),
            source,
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(Error::ProcessorFailed(Box::new(ProcessorFailure {
        family_name: context.family_name.to_owned(),
        processor: processor.jar.clone(),
        java: runtime.executable().display().to_string(),
        main_class,
        classpath,
        args: resolved_args,
        exit_status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })))
}

fn join_processor_classpath(
    context: &InstallerProcessorContext<'_>,
    processor: &ForgeInstallerProcessor,
) -> Result<String> {
    let entries = context.processor_classpath(processor)?;
    Ok(join_classpath(
        entries
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    ))
}

fn read_jar_main_class(path: &Path) -> Result<String> {
    let manifest = JarFile::new(path).by_name_string("META-INF/MANIFEST.MF")?;
    unfold_manifest_lines(manifest.as_str())
        .lines()
        .find_map(|line| line.strip_prefix("Main-Class: "))
        .map(str::trim)
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::JarMainClassMissing {
            path: path.to_path_buf(),
        })
}

fn unfold_manifest_lines(manifest: &str) -> String {
    manifest
        .lines()
        .fold(Vec::<String>::new(), |mut lines, line| {
            if let Some(last) = lines.last_mut()
                && let Some(continued) = line.strip_prefix(' ')
            {
                last.push_str(continued);
            } else {
                lines.push(line.to_owned());
            }
            lines
        })
        .join("\n")
}

async fn ensure_output_directories(
    context: &InstallerProcessorContext<'_>,
    processor: &ForgeInstallerProcessor,
) -> Result<()> {
    let mut parents = HashSet::new();
    for output in processor.outputs.keys() {
        let path = PathBuf::from(context.resolve_argument(output).await?);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            parents.insert(parent.to_path_buf());
        }
    }

    for parent in parents {
        create_dir_all(parent).await?;
    }

    Ok(())
}
