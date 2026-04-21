use anyhow::{Context, Result, bail};

pub fn parse_argument_string(argument_string: &str) -> Result<Vec<String>> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut chars = argument_string.chars().peekable();
    let mut active_quote: Option<char> = None;

    while let Some(character) = chars.next() {
        match active_quote {
            Some(quote) => match character {
                '\\' => {
                    let escaped = chars.next().context("trailing escape in argument string")?;
                    current.push(escaped);
                }
                value if value == quote => {
                    active_quote = None;
                }
                _ => current.push(character),
            },
            None => match character {
                '"' | '\'' => {
                    active_quote = Some(character);
                }
                '\\' => {
                    let escaped = chars.next().context("trailing escape in argument string")?;
                    current.push(escaped);
                }
                value if value.is_whitespace() => {
                    if !current.is_empty() {
                        arguments.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(character),
            },
        }
    }

    if let Some(quote) = active_quote {
        bail!("unterminated quoted argument string with quote {}", quote);
    }

    if !current.is_empty() {
        arguments.push(current);
    }

    Ok(arguments)
}
