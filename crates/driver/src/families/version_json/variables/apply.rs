use std::collections::HashMap;

use super::LauncherVariables;

impl LauncherVariables {
    pub fn apply(&self, args: &[String]) -> Vec<String> {
        args.iter()
            .map(|value| replace_variables(value, self, None))
            .collect()
    }

    pub fn apply_with(
        &self,
        args: &[String],
        extra_variables: &HashMap<String, String>,
    ) -> Vec<String> {
        args.iter()
            .map(|value| replace_variables(value, self, Some(extra_variables)))
            .collect()
    }
}

fn replace_variables(
    value: &str,
    variables: &LauncherVariables,
    extra_variables: Option<&HashMap<String, String>>,
) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find('}') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let key = &after_open[..end];
        let replacement = extra_variables
            .and_then(|extra| extra.get(key).map(String::as_str))
            .or_else(|| variables.get(key));
        match replacement {
            Some(replacement) => output.push_str(replacement),
            None => output.push_str(&rest[start..start + 2 + end + 1]),
        }
        rest = &after_open[end + 1..];
    }

    output.push_str(rest);
    output
}
