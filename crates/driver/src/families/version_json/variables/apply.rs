use std::collections::HashMap;

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

        result.extend(args.iter().map(|value| replace_variables(value, &data)));

        Ok(result)
    }
}

fn replace_variables(value: &str, data: &HashMap<String, String>) -> String {
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
        match data.get(key) {
            Some(replacement) => output.push_str(replacement),
            None => output.push_str(&rest[start..start + 2 + end + 1]),
        }
        rest = &after_open[end + 1..];
    }

    output.push_str(rest);
    output
}
