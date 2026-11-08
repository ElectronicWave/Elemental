use std::{
    fmt::{self, Display},
    path::PathBuf,
};

use anyhow::Result;
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::{
        catalog::{GameVersion, LoaderRelease},
        descriptors::FABRIC_DRIVER,
        driver::DriverDescriptor,
    },
    launcher::{DriverCatalog, Instance},
};
use inquire::{InquireError, Select};

#[derive(Clone, Copy, Debug)]
pub(super) struct DriverChoice {
    pub(super) descriptor: DriverDescriptor,
}

impl Display for DriverChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({})",
            self.descriptor.name, self.descriptor.id
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RuntimeChoice {
    Auto,
    Java { major: usize, path: PathBuf },
}

impl Display for RuntimeChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => formatter.write_str("Auto"),
            Self::Java { major, path } => {
                write!(formatter, "Java {major} | {}", path.display())
            }
        }
    }
}

pub(super) struct InstanceChoice {
    pub(super) instance: Instance,
}

impl Display for InstanceChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let game_version = self
            .instance
            .driver
            .game_version
            .as_ref()
            .map_or("<unknown>", MinecraftVersionId::as_str);
        let loader_version = self
            .instance
            .driver
            .driver_version
            .as_deref()
            .unwrap_or("<unknown>");

        write!(
            formatter,
            "{} | {} | game {} | loader {}",
            self.instance.instance_name,
            self.instance.driver.driver.name,
            game_version,
            loader_version
        )
    }
}

pub(super) struct GameVersionChoice {
    pub(super) version: GameVersion,
}

impl Display for GameVersionChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.version.id)?;
        if let Some(stream) = self.version.stream.as_deref() {
            write!(formatter, " [{stream}]")?;
        }
        if let Some(description) = self.version.description.as_deref() {
            write!(formatter, " {description}")?;
        }
        Ok(())
    }
}

pub(super) struct LoaderVersionChoice {
    pub(super) release: LoaderRelease,
}

impl Display for LoaderVersionChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.release.loader_version)?;
        if let Some(stream) = self.release.stream.as_deref() {
            write!(formatter, " [{stream}]")?;
        }
        if let Some(published_at) = self.release.published_at.as_deref() {
            write!(formatter, " {published_at}")?;
        }
        if let Some(description) = self.release.description.as_deref() {
            write!(formatter, " {description}")?;
        }
        Ok(())
    }
}

pub(super) fn prompt_or_back<T>(result: std::result::Result<T, InquireError>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn prompt_catalog<T>(
    message: &'static str,
    choices: Vec<T>,
    starting_cursor: usize,
) -> Result<Option<T>>
where
    T: Display,
{
    prompt_or_back(
        Select::new(message, choices)
            .with_starting_cursor(starting_cursor)
            .with_help_message("Type to filter the catalog. Esc goes back")
            .prompt(),
    )
}

pub(super) fn driver_choices() -> Vec<DriverChoice> {
    DriverCatalog::supported_drivers()
        .map(|descriptor| DriverChoice { descriptor })
        .collect()
}

pub(super) fn fabric_starting_cursor() -> usize {
    DriverCatalog::supported_drivers()
        .position(|descriptor| descriptor == FABRIC_DRIVER)
        .unwrap_or(0)
}

pub(super) fn driver_starting_cursor(driver: DriverDescriptor) -> usize {
    DriverCatalog::supported_drivers()
        .position(|descriptor| descriptor == driver)
        .unwrap_or_else(fabric_starting_cursor)
}
