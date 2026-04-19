use anyhow::Result;
use demo::{demo_install_spec, demo_launch_spec, launch_demo};

#[tokio::main]
async fn main() -> Result<()> {
    let launch_summary = launch_demo(&demo_install_spec(), &demo_launch_spec()).await?;
    println!(
        "Using java executable: {}",
        launch_summary.runtime_executable.to_string_lossy()
    );
    println!("{}", launch_summary.install_summary.render());
    println!("process exited with: {}", launch_summary.exit_status);
    Ok(())
}
