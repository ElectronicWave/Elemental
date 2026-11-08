use std::collections::HashSet;

use anyhow::Result;
use elemental::core::runtime::{RuntimeValidationMode, distribution::Distribution};
use inquire::Select;

use super::choices::{RuntimeChoice, prompt_or_back};

pub(super) struct RuntimeSelection {
    pub(super) major_version: Option<usize>,
    pub(super) validation: RuntimeValidationMode,
}

pub(super) async fn prompt(
    default_major_version: Option<usize>,
    current_runtime: &mut Option<RuntimeChoice>,
) -> Result<Option<RuntimeSelection>> {
    let choices = runtime_choices().await;
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

    Ok(Some(RuntimeSelection {
        major_version: match choice {
            RuntimeChoice::Auto => default_major_version,
            RuntimeChoice::Java { major, .. } => Some(major),
        },
        validation: RuntimeValidationMode::Strict,
    }))
}

async fn runtime_choices() -> Vec<RuntimeChoice> {
    let mut seen_majors = HashSet::new();
    let mut runtimes = Vec::new();
    for distribution in Distribution::cached().await {
        let Some(major) = distribution.java_major_version() else {
            continue;
        };
        if !seen_majors.insert(major) {
            continue;
        }
        runtimes.push((major, distribution.path));
    }
    runtimes.sort_unstable_by_key(|(major, _)| *major);

    let mut choices = Vec::with_capacity(runtimes.len() + 1);
    choices.push(RuntimeChoice::Auto);
    choices.extend(
        runtimes
            .into_iter()
            .map(|(major, path)| RuntimeChoice::Java { major, path }),
    );
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
