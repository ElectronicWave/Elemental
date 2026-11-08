use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use elemental::{
    core::minecraft::MinecraftVersionId,
    driver::{
        catalog::{GameVersion, LoaderRelease},
        loader_version::LoaderVersionId,
    },
    launcher::{DriverCatalog, LaunchOptions},
};
use inquire::{Select, Text};

use crate::{
    config::{LaunchpadConfig, build_spec},
    defaults::{DriverDefaults, default_instance_name, driver_defaults},
    progress,
};

use super::{
    choices::{
        DriverChoice, GameVersionChoice, LoaderVersionChoice, RuntimeChoice, driver_choices,
        driver_starting_cursor, fabric_starting_cursor, prompt_or_back,
    },
    runtime,
};

#[derive(Clone, Copy, Debug)]
enum NewInstanceStep {
    Driver,
    GameVersion,
    LoaderVersion,
    InstanceName,
}

enum StepResult {
    Next(NewInstanceStep),
    Back(NewInstanceStep),
    Finished(Box<LaunchpadConfig>),
    Cancelled,
}

struct NewInstanceWizard {
    storage_root: PathBuf,
    defaults: Option<DriverDefaults>,
    catalog: Option<DriverCatalog>,
    game_versions: Vec<GameVersion>,
    game_version: Option<MinecraftVersionId>,
    loader_version: Option<LoaderVersionId>,
    instance_name: Option<String>,
    runtime_choice: Option<RuntimeChoice>,
}

impl NewInstanceWizard {
    fn new(storage_root: PathBuf) -> Self {
        Self {
            storage_root,
            defaults: None,
            catalog: None,
            game_versions: Vec::new(),
            game_version: None,
            loader_version: None,
            instance_name: None,
            runtime_choice: None,
        }
    }

    async fn run(mut self) -> Result<Option<Box<LaunchpadConfig>>> {
        let mut step = NewInstanceStep::Driver;
        loop {
            let result = match step {
                NewInstanceStep::Driver => self.prompt_driver()?,
                NewInstanceStep::GameVersion => self.prompt_game_version().await?,
                NewInstanceStep::LoaderVersion => self.prompt_loader_version().await?,
                NewInstanceStep::InstanceName => self.prompt_instance_name().await?,
            };

            match result {
                StepResult::Next(next) => step = next,
                StepResult::Back(previous) => step = previous,
                StepResult::Finished(config) => return Ok(Some(config)),
                StepResult::Cancelled => return Ok(None),
            }
        }
    }

    fn prompt_driver(&mut self) -> Result<StepResult> {
        let starting_cursor = self
            .catalog
            .as_ref()
            .map_or_else(fabric_starting_cursor, |catalog| {
                driver_starting_cursor(catalog.driver())
            });
        let result = Select::new("Choose a driver", driver_choices())
            .with_starting_cursor(starting_cursor)
            .with_help_message("Esc goes back to the main menu")
            .prompt();
        let Some(driver) = prompt_or_back(result)? else {
            return Ok(StepResult::Cancelled);
        };

        self.select_driver(driver)?;
        Ok(StepResult::Next(NewInstanceStep::GameVersion))
    }

    async fn prompt_game_version(&mut self) -> Result<StepResult> {
        let defaults = self
            .defaults
            .context("driver defaults are missing from the wizard state")?;
        self.ensure_game_versions().await?;

        let starting_cursor = self.game_version_cursor(defaults);
        let result = Select::new(
            "Minecraft version",
            self.game_versions
                .iter()
                .cloned()
                .map(|version| GameVersionChoice { version })
                .collect(),
        )
        .with_starting_cursor(starting_cursor)
        .with_help_message("Type to filter the catalog. Esc goes back")
        .prompt();
        let Some(choice) = prompt_or_back(result)? else {
            return Ok(StepResult::Back(NewInstanceStep::Driver));
        };

        let game_version_changed = self.game_version.as_ref() != Some(&choice.version.id);
        if game_version_changed {
            self.loader_version = None;
            self.instance_name = None;
        }
        self.game_version = Some(choice.version.id);

        let next = if defaults.loader_version.is_some() {
            NewInstanceStep::LoaderVersion
        } else {
            NewInstanceStep::InstanceName
        };
        Ok(StepResult::Next(next))
    }

    async fn prompt_loader_version(&mut self) -> Result<StepResult> {
        let defaults = self
            .defaults
            .context("driver defaults are missing from the wizard state")?;
        let game_version = self
            .game_version
            .clone()
            .context("Minecraft version is missing from the wizard state")?;
        let catalog = self
            .catalog
            .as_ref()
            .context("driver catalog is missing from the wizard state")?;
        let default_loader_version = defaults
            .loader_version
            .context("loader version default is missing from the wizard state")?;
        let message = format!(
            "Fetching {} loader versions for {}",
            catalog.driver().name,
            game_version
        );
        let loader_releases =
            progress::with_spinner(&message, catalog.loader_versions(&game_version)).await?;
        if loader_releases.is_empty() {
            bail!(
                "No {} loader versions are available for Minecraft {}",
                catalog.driver().name,
                game_version
            );
        }

        let starting_cursor = self.loader_version_cursor(&loader_releases, default_loader_version);
        let result = Select::new(
            "Loader version",
            loader_releases
                .into_iter()
                .map(|release| LoaderVersionChoice { release })
                .collect(),
        )
        .with_starting_cursor(starting_cursor)
        .with_help_message("Type to filter the catalog. Esc goes back")
        .prompt();
        let Some(choice) = prompt_or_back(result)? else {
            return Ok(StepResult::Back(NewInstanceStep::GameVersion));
        };

        self.loader_version = Some(choice.release.loader_version);
        Ok(StepResult::Next(NewInstanceStep::InstanceName))
    }

    async fn prompt_instance_name(&mut self) -> Result<StepResult> {
        let driver = self
            .catalog
            .as_ref()
            .context("driver catalog is missing from the wizard state")?
            .driver();
        let defaults = self
            .defaults
            .context("driver defaults are missing from the wizard state")?;
        let game_version = self
            .game_version
            .clone()
            .context("Minecraft version is missing from the wizard state")?;
        let default_name = default_instance_name(defaults, game_version.as_str());
        let prompt = if let Some(name) = self.instance_name.as_deref() {
            Text::new("Instance name")
                .with_initial_value(name)
                .with_help_message("Esc goes back")
        } else {
            Text::new("Instance name")
                .with_default(&default_name)
                .with_help_message("Enter accepts the generated name. Esc goes back")
        };
        let Some(instance_name) = prompt_or_back(prompt.prompt())? else {
            return Ok(StepResult::Back(previous_step(defaults)));
        };
        self.instance_name = Some(instance_name.clone());

        let Some(runtime) =
            runtime::prompt(defaults.runtime_major_version, &mut self.runtime_choice).await?
        else {
            return Ok(StepResult::Back(NewInstanceStep::InstanceName));
        };

        let spec = build_spec(driver, game_version, self.loader_version.clone())?;
        let options = LaunchOptions {
            runtime_major_version: runtime.major_version,
            runtime_validation: runtime.validation,
            ..LaunchOptions::default()
        };
        Ok(StepResult::Finished(Box::new(LaunchpadConfig {
            spec,
            options,
            runtime_paths: Vec::new(),
            storage_root: self.storage_root.clone(),
            instance_name,
            local_only: false,
        })))
    }

    fn select_driver(&mut self, choice: DriverChoice) -> Result<()> {
        let driver_changed = self
            .catalog
            .as_ref()
            .is_none_or(|catalog| catalog.driver() != choice.descriptor);
        if driver_changed {
            let defaults = driver_defaults(choice.descriptor).with_context(|| {
                format!("{} launchpad defaults are missing", choice.descriptor.name)
            })?;
            let catalog = DriverCatalog::for_driver(choice.descriptor)?;

            self.defaults = Some(defaults);
            self.catalog = Some(catalog);
            self.game_versions.clear();
            self.game_version = None;
            self.loader_version = None;
            self.instance_name = None;
        }

        Ok(())
    }

    async fn ensure_game_versions(&mut self) -> Result<()> {
        if !self.game_versions.is_empty() {
            return Ok(());
        }

        let catalog = self
            .catalog
            .as_ref()
            .context("driver catalog is missing from the wizard state")?;
        let driver = catalog.driver();
        let message = format!("Fetching {} Minecraft versions", driver.name);
        let game_versions = progress::with_spinner(&message, catalog.game_versions()).await?;
        if game_versions.is_empty() {
            bail!("No Minecraft versions are available for {}", driver.name);
        }

        self.game_versions = game_versions;
        Ok(())
    }

    fn game_version_cursor(&self, defaults: DriverDefaults) -> usize {
        self.game_version
            .as_ref()
            .and_then(|selected| {
                self.game_versions
                    .iter()
                    .position(|version| &version.id == selected)
            })
            .or_else(|| {
                self.game_versions
                    .iter()
                    .position(|version| version.id.as_str() == defaults.game_version)
            })
            .unwrap_or(0)
    }

    fn loader_version_cursor(
        &self,
        loader_releases: &[LoaderRelease],
        default_loader_version: &str,
    ) -> usize {
        self.loader_version
            .as_ref()
            .and_then(|selected| {
                loader_releases
                    .iter()
                    .position(|release| &release.loader_version == selected)
            })
            .or_else(|| {
                loader_releases
                    .iter()
                    .position(|release| release.loader_version.as_str() == default_loader_version)
            })
            .unwrap_or(0)
    }
}

pub(super) async fn prompt(storage_root: PathBuf) -> Result<Option<Box<LaunchpadConfig>>> {
    NewInstanceWizard::new(storage_root).run().await
}

fn previous_step(defaults: DriverDefaults) -> NewInstanceStep {
    if defaults.loader_version.is_some() {
        NewInstanceStep::LoaderVersion
    } else {
        NewInstanceStep::GameVersion
    }
}
