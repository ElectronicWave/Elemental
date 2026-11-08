use std::path::PathBuf;

use crate::config::LaunchpadCommand;

pub enum Invocation {
    Command(LaunchpadCommand),
    Interactive {
        storage_root: PathBuf,
        local_only: bool,
    },
}
