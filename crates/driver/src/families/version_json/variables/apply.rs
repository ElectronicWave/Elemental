use std::collections::HashMap;

use regex::Regex;

use crate::error::Result;

use super::LauncherVariables;

impl LauncherVariables {
    pub fn apply_mut(&self, args: &mut [String]) -> Result<()> {
        let applied = self.apply(args.to_vec())?;
        args.clone_from_slice(&applied);
        Ok(())
    }

    pub fn apply(&self, args: Vec<String>) -> Result<Vec<String>> {
        self.apply_with(args, &HashMap::new())
    }

    pub fn apply_with(
        &self,
        args: Vec<String>,
        extra_variables: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let mut result = Vec::with_capacity(args.len());
        let mut data = self.hashmap()?;
        data.extend(extra_variables.clone());
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;

        result.extend(
            args.iter()
                .map(|value| replace_variables(value, &regex, &data)),
        );

        Ok(result)
    }
}

fn replace_variables(value: &str, regex: &Regex, data: &HashMap<String, String>) -> String {
    let mut replaced = value.to_owned();
    for variable in regex.captures_iter(value) {
        if let Some(key) = variable.get(1).map(|item| item.as_str())
            && let Some(replacement) = data.get(key)
        {
            replaced = replaced.replace(&format!("${{{key}}}"), replacement);
        }
    }
    replaced
}
