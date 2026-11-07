use crate::error::{Error, Result};

pub(crate) fn validate_component(value: &str, label: &str, notation: &str) -> Result<()> {
    let rejected = value.is_empty()
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', ':', '?', '#'])
        || value.chars().any(char::is_control);

    if rejected {
        return Err(Error::InvalidNotation {
            label: label.to_owned(),
            notation: notation.to_owned(),
        });
    }

    Ok(())
}
