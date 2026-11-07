use crate::error::{Error, Result};

pub fn parse_argument_string(argument_string: &str) -> Result<Vec<String>> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut chars = argument_string.chars().peekable();
    let mut active_quote: Option<char> = None;

    while let Some(character) = chars.next() {
        if character == '\\' {
            current.push(chars.next().ok_or(Error::TrailingArgumentEscape)?);
            continue;
        }

        if let Some(quote) = active_quote {
            if character == quote {
                active_quote = None;
            } else {
                current.push(character);
            }
            continue;
        }

        match character {
            '"' | '\'' => active_quote = Some(character),
            value if value.is_whitespace() => {
                if !current.is_empty() {
                    arguments.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }

    if let Some(quote) = active_quote {
        return Err(Error::UnterminatedArgumentQuote { quote });
    }

    if !current.is_empty() {
        arguments.push(current);
    }

    Ok(arguments)
}
