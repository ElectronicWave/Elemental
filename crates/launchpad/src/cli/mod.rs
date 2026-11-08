use std::path::PathBuf;

use anyhow::Result;

use crate::{config::LaunchpadCommand, invocation::Invocation};

mod args;
mod launch;

pub use args::Cli;

impl Cli {
    pub fn into_invocation(self) -> Result<Invocation> {
        let storage_root = self
            .storage_root
            .unwrap_or_else(|| PathBuf::from(".minecraft"));

        if matches!(self.command, Some(args::LaunchpadCliCommand::List)) {
            return Ok(Invocation::Command(LaunchpadCommand::ListInstances {
                storage_root,
            }));
        }

        if self.launch.is_empty() {
            return Ok(Invocation::Interactive {
                storage_root,
                local_only: self.local_only,
            });
        }

        Ok(Invocation::Command(LaunchpadCommand::Launch(Box::new(
            launch::build(storage_root, self.local_only, self.launch)?,
        ))))
    }
}
