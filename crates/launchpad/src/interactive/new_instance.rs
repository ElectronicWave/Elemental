use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use elemental::{
    core::minecraft::MinecraftVersionId,
    core::runtime::RuntimeResolver,
    driver::{
        catalog::{GameVersion, LoaderRelease},
        driver::DriverDescriptor,
        loader_version::LoaderVersionId,
    },
    launcher::{DriverCatalog, LaunchOptions, RuntimeOptions},
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
        driver_starting_cursor, fabric_starting_cursor, prompt_catalog, prompt_or_back,
    },
    runtime,
};

#[derive(Clone, Copy, Debug)]
enum NewInstanceStep {
    Driver,
    GameVersion,
    LoaderVersion,
    InstanceName,
    Runtime,
}

enum StepOutcome {
    Step(NewInstanceStep),
    Finished(Box<LaunchpadConfig>),
    Cancelled,
}

struct DriverContext {
    descriptor: DriverDescriptor,
    defaults: DriverDefaults,
    catalog: DriverCatalog,
    game_versions: Vec<GameVersion>,
}

impl DriverContext {
    fn new(descriptor: DriverDescriptor) -> Result<Self> {
        let defaults = driver_defaults(descriptor)
            .with_context(|| format!("{} launchpad defaults are missing", descriptor.name))?;
        let catalog = DriverCatalog::for_driver(descriptor)?;

        Ok(Self {
            descriptor,
            defaults,
            catalog,
            game_versions: Vec::new(),
        })
    }

    async fn load_game_versions_once(&mut self) -> Result<()> {
        if !self.game_versions.is_empty() {
            return Ok(());
        }

        let message = format!("Fetching {} Minecraft versions", self.descriptor.name);
        let game_versions = progress::with_spinner(&message, self.catalog.game_versions()).await?;
        if game_versions.is_empty() {
            bail!(
                "No Minecraft versions are available for {}",
                self.descriptor.name
            );
        }

        self.game_versions = game_versions;
        Ok(())
    }

    fn game_version_cursor(&self, selected: Option<&MinecraftVersionId>) -> usize {
        selected
            .and_then(|selected| {
                self.game_versions
                    .iter()
                    .position(|version| &version.id == selected)
            })
            .or_else(|| {
                self.game_versions
                    .iter()
                    .position(|version| version.id.as_str() == self.defaults.game_version)
            })
            .unwrap_or(0)
    }
}

struct NewInstanceWizard {
    storage_root: PathBuf,
    runtime_resolver: RuntimeResolver,
    driver: Option<DriverContext>,
    game_version: Option<MinecraftVersionId>,
    loader_version: Option<LoaderVersionId>,
    instance_name: Option<String>,
    runtime_choice: Option<RuntimeChoice>,
}

impl NewInstanceWizard {
    fn new(storage_root: PathBuf, runtime_resolver: RuntimeResolver) -> Self {
        Self {
            storage_root,
            runtime_resolver,
            driver: None,
            game_version: None,
            loader_version: None,
            instance_name: None,
            runtime_choice: None,
        }
    }

    async fn run(mut self) -> Result<Option<Box<LaunchpadConfig>>> {
        let mut step = NewInstanceStep::Driver;
        loop {
            match step.prompt(&mut self).await? {
                StepOutcome::Step(next) => step = next,
                StepOutcome::Finished(config) => return Ok(Some(config)),
                StepOutcome::Cancelled => return Ok(None),
            }
        }
    }

    fn prompt_driver(&mut self) -> Result<StepOutcome> {
        let starting_cursor = self
            .driver
            .as_ref()
            .map_or_else(fabric_starting_cursor, |driver| {
                driver_starting_cursor(driver.descriptor)
            });
        let result = Select::new("Choose a driver", driver_choices())
            .with_starting_cursor(starting_cursor)
            .with_help_message("Esc goes back to the main menu")
            .prompt();
        let Some(driver) = prompt_or_back(result)? else {
            return Ok(StepOutcome::Cancelled);
        };

        self.select_driver(driver)?;
        Ok(StepOutcome::Step(NewInstanceStep::GameVersion))
    }

    fn select_driver(&mut self, choice: DriverChoice) -> Result<()> {
        if self
            .driver
            .as_ref()
            .is_some_and(|driver| driver.descriptor == choice.descriptor)
        {
            return Ok(());
        }

        self.driver = Some(DriverContext::new(choice.descriptor)?);
        self.game_version = None;
        self.loader_version = None;
        self.instance_name = None;
        Ok(())
    }

    async fn prompt_game_version(&mut self) -> Result<StepOutcome> {
        let driver = self
            .driver
            .as_mut()
            .context("driver is missing from the wizard state")?;
        driver.load_game_versions_once().await?;

        let defaults = driver.defaults;
        let starting_cursor = driver.game_version_cursor(self.game_version.as_ref());
        let choices = driver
            .game_versions
            .iter()
            .cloned()
            .map(|version| GameVersionChoice { version })
            .collect();
        let Some(choice) = prompt_catalog("Minecraft version", choices, starting_cursor)? else {
            return Ok(StepOutcome::Step(NewInstanceStep::Driver));
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
        Ok(StepOutcome::Step(next))
    }

    async fn prompt_loader_version(&mut self) -> Result<StepOutcome> {
        let driver = self
            .driver
            .as_ref()
            .context("driver is missing from the wizard state")?;
        let game_version = self
            .game_version
            .as_ref()
            .context("Minecraft version is missing from the wizard state")?;
        let default_loader_version = driver
            .defaults
            .loader_version
            .context("loader version default is missing from the wizard state")?;
        let message = format!(
            "Fetching {} loader versions for {}",
            driver.descriptor.name, game_version
        );
        let loader_releases =
            progress::with_spinner(&message, driver.catalog.loader_versions(game_version)).await?;
        if loader_releases.is_empty() {
            bail!(
                "No {} loader versions are available for Minecraft {}",
                driver.descriptor.name,
                game_version
            );
        }

        let starting_cursor = self.loader_version_cursor(&loader_releases, default_loader_version);
        let choices = loader_releases
            .into_iter()
            .map(|release| LoaderVersionChoice { release })
            .collect();
        let Some(choice) = prompt_catalog("Loader version", choices, starting_cursor)? else {
            return Ok(StepOutcome::Step(NewInstanceStep::GameVersion));
        };

        self.loader_version = Some(choice.release.loader_version);
        Ok(StepOutcome::Step(NewInstanceStep::InstanceName))
    }

    async fn prompt_instance_name(&mut self) -> Result<StepOutcome> {
        let defaults = self
            .driver
            .as_ref()
            .context("driver is missing from the wizard state")?
            .defaults;
        let game_version = self
            .game_version
            .as_ref()
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
            let previous = if defaults.loader_version.is_some() {
                NewInstanceStep::LoaderVersion
            } else {
                NewInstanceStep::GameVersion
            };
            return Ok(StepOutcome::Step(previous));
        };
        self.instance_name = Some(instance_name);
        Ok(StepOutcome::Step(NewInstanceStep::Runtime))
    }

    async fn prompt_runtime(&mut self) -> Result<StepOutcome> {
        let driver = self
            .driver
            .as_ref()
            .context("driver is missing from the wizard state")?;
        let defaults = driver.defaults;
        let descriptor = driver.descriptor;
        let Some(runtime) = runtime::prompt(
            &self.runtime_resolver,
            defaults.runtime_major_version,
            &mut self.runtime_choice,
        )
        .await?
        else {
            return Ok(StepOutcome::Step(NewInstanceStep::InstanceName));
        };

        let game_version = self
            .game_version
            .take()
            .context("Minecraft version is missing from the wizard state")?;
        let spec = build_spec(descriptor, game_version, self.loader_version.take())?;
        let instance_name = self
            .instance_name
            .take()
            .context("instance name is missing from the wizard state")?;
        let runtime = RuntimeOptions {
            runtime_major_version: runtime.major_version,
            runtime_override: runtime.distribution,
            runtime_validation: runtime.validation,
        };
        let options = LaunchOptions::default();
        Ok(StepOutcome::Finished(Box::new(LaunchpadConfig {
            spec,
            runtime,
            options,
            runtime_paths: Vec::new(),
            storage_root: self.storage_root.clone(),
            instance_name,
            local_only: false,
        })))
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

impl NewInstanceStep {
    async fn prompt(self, wizard: &mut NewInstanceWizard) -> Result<StepOutcome> {
        match self {
            Self::Driver => wizard.prompt_driver(),
            Self::GameVersion => wizard.prompt_game_version().await,
            Self::LoaderVersion => wizard.prompt_loader_version().await,
            Self::InstanceName => wizard.prompt_instance_name().await,
            Self::Runtime => wizard.prompt_runtime().await,
        }
    }
}

pub(super) async fn prompt(
    storage_root: PathBuf,
    runtime_resolver: RuntimeResolver,
) -> Result<Option<Box<LaunchpadConfig>>> {
    NewInstanceWizard::new(storage_root, runtime_resolver)
        .run()
        .await
}
