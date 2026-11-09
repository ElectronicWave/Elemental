use elemental_schema::mojang::piston::{
    ContinuousArgument, PistonMetaData, PistonMetaGenericArgument, PistonMetaRuleArgument,
};

use crate::error::Result;

use super::{PistonMetaRulesExt, VersionJsonRuleContext, parse_argument_string};

mod libraries;

pub use libraries::PistonMetaLibrariesExt;

pub trait PistonMetaDataExt {
    fn game_version(&self) -> &str;
    fn jvm_arguments(&self, context: &VersionJsonRuleContext) -> Vec<String>;
    fn game_arguments(&self, context: &VersionJsonRuleContext) -> Result<Vec<String>>;
}

impl PistonMetaDataExt for PistonMetaData {
    fn game_version(&self) -> &str {
        self.inherits_from
            .as_deref()
            .or(self.client_version.as_deref())
            .unwrap_or(self.id.as_str())
    }

    fn jvm_arguments(&self, context: &VersionJsonRuleContext) -> Vec<String> {
        self.arguments
            .as_ref()
            .map(|arguments| collect_arguments(&arguments.jvm, context))
            .unwrap_or_default()
    }

    fn game_arguments(&self, context: &VersionJsonRuleContext) -> Result<Vec<String>> {
        if let Some(arguments) = &self.arguments {
            let game_arguments = collect_arguments(&arguments.game, context);
            if !game_arguments.is_empty() {
                return Ok(game_arguments);
            }
        }

        parse_argument_string(self.minecraft_arguments.as_deref().unwrap_or_default())
    }
}

fn collect_arguments(
    arguments: &[PistonMetaGenericArgument],
    context: &VersionJsonRuleContext,
) -> Vec<String> {
    let mut result = Vec::new();

    for argument in arguments {
        match argument {
            PistonMetaGenericArgument::Plain(value) => {
                result.push(value.clone());
            }
            PistonMetaGenericArgument::Rule(rule_argument) => {
                if !rule_argument.rules.as_slice().are_allowed(context) {
                    continue;
                }

                extend_argument_values(&mut result, rule_argument);
            }
        }
    }

    result
}

fn extend_argument_values(target: &mut Vec<String>, argument: &PistonMetaRuleArgument) {
    if let Some(value) = &argument.value {
        match value {
            ContinuousArgument::Single(value) => target.push(value.clone()),
            ContinuousArgument::Multi(values) => target.extend(values.iter().cloned()),
        }
    }
}
