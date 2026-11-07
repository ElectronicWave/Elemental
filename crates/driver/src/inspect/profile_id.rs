#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileIdPattern {
    separator: &'static str,
    form: ProfileIdForm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileIdForm {
    GameAndLoader,
    LoaderPrefixed,
}

impl ProfileIdPattern {
    pub const fn new(separator: &'static str) -> Self {
        Self {
            separator,
            form: ProfileIdForm::GameAndLoader,
        }
    }

    pub const fn loader_prefixed(separator: &'static str) -> Self {
        Self {
            separator,
            form: ProfileIdForm::LoaderPrefixed,
        }
    }

    pub fn build(self, game_version: &str, loader_version: &str) -> String {
        match self.form {
            ProfileIdForm::GameAndLoader => {
                format!("{game_version}{}{loader_version}", self.separator)
            }
            ProfileIdForm::LoaderPrefixed => format!("{}{loader_version}", self.separator),
        }
    }

    pub fn loader_version(self, metadata_id: &str) -> Option<String> {
        let loader_version = match self.form {
            ProfileIdForm::GameAndLoader => metadata_id
                .split_once(self.separator)
                .map(|(_, loader_version)| loader_version),
            ProfileIdForm::LoaderPrefixed => metadata_id.strip_prefix(self.separator),
        };

        loader_version
            .filter(|loader_version| !loader_version.is_empty())
            .map(ToOwned::to_owned)
    }
}
