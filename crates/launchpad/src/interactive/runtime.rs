use anyhow::Result;
use elemental::core::runtime::{
    RuntimeResolver, RuntimeValidationMode, distribution::Distribution,
};
use inquire::Select;

use super::choices::{RuntimeChoice, prompt_or_back};

pub(super) struct RuntimeSelection {
    pub(super) major_version: Option<usize>,
    pub(super) distribution: Option<Distribution>,
    pub(super) validation: RuntimeValidationMode,
}

pub(super) async fn prompt(
    runtime_resolver: &RuntimeResolver,
    default_major_version: Option<usize>,
    current_runtime: &mut Option<RuntimeChoice>,
) -> Result<Option<RuntimeSelection>> {
    let choices = runtime_choices(runtime_resolver).await;
    let starting_cursor = runtime_starting_cursor(current_runtime.as_ref(), &choices);
    let help_message = match default_major_version {
        Some(major) => format!("Auto uses Java {major} from the driver default. Esc goes back"),
        None => "Auto uses the version metadata. Esc goes back".to_owned(),
    };
    let result = Select::new("Java runtime", choices)
        .with_starting_cursor(starting_cursor)
        .with_help_message(&help_message)
        .prompt();
    let Some(choice) = prompt_or_back(result)? else {
        return Ok(None);
    };
    *current_runtime = Some(choice.clone());

    let (major_version, distribution) = match choice {
        RuntimeChoice::Auto => (default_major_version, None),
        RuntimeChoice::Java {
            major,
            distribution,
        } => (Some(major), Some(distribution)),
    };

    Ok(Some(RuntimeSelection {
        major_version,
        distribution,
        validation: RuntimeValidationMode::Strict,
    }))
}

async fn runtime_choices(runtime_resolver: &RuntimeResolver) -> Vec<RuntimeChoice> {
    let distributions = runtime_resolver.distributions().await;
    let mut choices = Vec::with_capacity(distributions.len() + 1);
    choices.push(RuntimeChoice::Auto);
    choices.extend(distributions.into_iter().filter_map(|distribution| {
        distribution
            .java_major_version()
            .map(|major| RuntimeChoice::Java {
                major,
                distribution,
            })
    }));
    choices.sort_by_key(RuntimeChoice::major_version);
    choices
}

fn runtime_starting_cursor(
    current_runtime: Option<&RuntimeChoice>,
    choices: &[RuntimeChoice],
) -> usize {
    current_runtime
        .and_then(|selected| choices.iter().position(|choice| choice == selected))
        .unwrap_or(0)
}
