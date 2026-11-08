use std::{
    fmt::{self, Display},
    path::PathBuf,
};

use anyhow::Result;
use elemental::launcher::Launcher;
use inquire::Select;

use crate::config::LaunchpadCommand;

use choices::prompt_or_back;

mod choices;
mod installed;
mod new_instance;
mod runtime;

#[derive(Clone, Copy, Debug)]
enum LaunchpadAction {
    NewInstance,
    InstalledInstance,
    ListInstances,
}

impl Display for LaunchpadAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NewInstance => "Prepare and launch a new instance",
            Self::InstalledInstance => "Launch an installed instance",
            Self::ListInstances => "List installed instances",
        })
    }
}

impl LaunchpadAction {
    fn prompt(local_only: bool) -> Result<Option<Self>> {
        prompt_or_back(
            Select::new("What would you like to do?", action_choices(local_only))
                .with_help_message("Arrow keys move, Enter confirms, Esc exits")
                .prompt(),
        )
    }

    async fn execute(
        self,
        storage_root: PathBuf,
        launcher: &Launcher,
    ) -> Result<Option<LaunchpadCommand>> {
        match self {
            Self::NewInstance => Ok(new_instance::prompt(
                storage_root,
                launcher.runtime_resolver().clone(),
            )
            .await?
            .map(LaunchpadCommand::Launch)),
            Self::InstalledInstance => installed::prompt(launcher).await,
            Self::ListInstances => Ok(Some(LaunchpadCommand::ListInstances { storage_root })),
        }
    }
}

pub async fn prompt_command(storage_root: PathBuf, local_only: bool) -> Result<LaunchpadCommand> {
    println!("Elemental launchpad interactive setup");
    println!("Storage root: {}", storage_root.display());
    let launcher = Launcher::builder()
        .storage_root(storage_root.clone())
        .build();

    loop {
        let Some(action) = LaunchpadAction::prompt(local_only)? else {
            return Ok(LaunchpadCommand::Exit);
        };

        if let Some(command) = action.execute(storage_root.clone(), &launcher).await? {
            return Ok(command);
        }
    }
}

fn action_choices(local_only: bool) -> Vec<LaunchpadAction> {
    if local_only {
        return vec![
            LaunchpadAction::InstalledInstance,
            LaunchpadAction::ListInstances,
        ];
    }

    vec![
        LaunchpadAction::NewInstance,
        LaunchpadAction::InstalledInstance,
        LaunchpadAction::ListInstances,
    ]
}
