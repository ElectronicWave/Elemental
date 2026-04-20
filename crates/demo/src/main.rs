use std::{path::PathBuf, time::Instant};

use anyhow::Result;
use elemental::{
    core::{auth::authorizers::offline::OfflineAuthorizer, storage::Storage},
    driver::{
        drivers::{
            vanilla::{VanillaDriver, VanillaLaunchConfig},
            version_json::{BaseLayout, VersionJsonGameStorageExt},
        },
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let storage = Storage::new(PathBuf::from(".minecraft"), BaseLayout);
    let instance = storage.instance("MyGame-1.16.5".to_owned(), BaseLayout)?;
    let vanilla = VanillaDriver::with_defaults()?;
    let launch_config = VanillaLaunchConfig::new();
    let authorizer = OfflineAuthorizer {
        username: "Player".to_owned(),
    };

    let started_at = Instant::now();
    let prepared = vanilla.prepare(&instance, "1.16.5".to_owned()).await?;
    let prepare_elapsed = started_at.elapsed();

    let launched = vanilla.launch(prepared, &launch_config, authorizer).await?;
    let runtime_executable = launched.runtime.executable();
    let install_status = launched.prepared_version.install_status;
    let version_root = launched
        .prepared_version
        .resolved_version
        .version
        .path
        .clone();

    let mut child = launched.child;
    let exit_status = child.wait().await?;

    println!(
        "Using java executable: {}",
        runtime_executable.to_string_lossy()
    );
    println!("version root: {}", version_root.display());
    println!("install status: {:?}", install_status);
    println!("prepared in {}ms", prepare_elapsed.as_millis());
    println!("process exited with: {}", exit_status);

    Ok(())
}
