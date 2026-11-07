use std::collections::HashMap;

use regex::Regex;
use serde_json::Value;

use crate::error::Result;

use super::LauncherVariables;

impl LauncherVariables {
    pub fn apply_mut(&self, args: &mut [String]) -> Result<()> {
        let data = self.map()?;
        let regex = Regex::new(r#"\$\{(.*?)\}"#)?;

        for value in args.iter_mut() {
            let original = value.clone();
            let mut copied = original.clone();
            for variable in regex.captures_iter(&original) {
                if let Some(key) = variable.get(1).map(|item| item.as_str())
                    && let Some(Value::String(replacement)) = data.get(key)
                {
                    copied = copied.replace(&format!("${{{key}}}"), replacement);
                }
            }
            *value = copied;
        }

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

        for value in &args {
            let mut copied = value.clone();
            for variable in regex.captures_iter(value) {
                if let Some(key) = variable.get(1).map(|item| item.as_str())
                    && let Some(replacement) = data.get(key)
                {
                    copied = copied.replace(&format!("${{{key}}}"), replacement);
                }
            }
            result.push(copied);
        }

        Ok(result)
    }
}
