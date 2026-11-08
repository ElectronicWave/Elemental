use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::LauncherVariables;

static VARIABLE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\$\{(.*?)\}"#).expect("launcher variable regex is valid"));

impl LauncherVariables {
    pub fn apply(&self, args: Vec<String>) -> Vec<String> {
        self.apply_with(args, &HashMap::new())
    }

    pub fn apply_with(
        &self,
        args: Vec<String>,
        extra_variables: &HashMap<String, String>,
    ) -> Vec<String> {
        args.into_iter()
            .map(|value| self.replace_variables(value.as_str(), extra_variables))
            .collect()
    }

    fn replace_variables(&self, value: &str, extra_variables: &HashMap<String, String>) -> String {
        VARIABLE_PATTERN
            .replace_all(value, |captures: &regex::Captures<'_>| {
                let Some(key) = captures.get(1).map(|item| item.as_str()) else {
                    return captures[0].to_owned();
                };
                extra_variables
                    .get(key)
                    .map(String::as_str)
                    .or_else(|| self.value_for(key))
                    .map_or_else(|| captures[0].to_owned(), ToOwned::to_owned)
            })
            .into_owned()
    }
}
