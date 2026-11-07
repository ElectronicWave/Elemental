use super::super::error::MinecraftVersionParseError;

pub(super) fn parse_numeric_components(
    raw: &str,
    value: &str,
) -> Result<Vec<u32>, MinecraftVersionParseError> {
    value
        .split('.')
        .map(|component| parse_component(raw, component))
        .collect()
}

pub(super) fn parse_component(
    raw: &str,
    component: &str,
) -> Result<u32, MinecraftVersionParseError> {
    component
        .parse()
        .map_err(|_| MinecraftVersionParseError::InvalidNumber {
            raw: raw.to_owned(),
            component: component.to_owned(),
        })
}

pub(super) fn full_year(year: u32) -> Option<u32> {
    match year {
        25..=99 => Some(2000 + year),
        2000..=9999 => Some(year),
        _ => None,
    }
}

pub(crate) fn normalize_year(year: u32) -> u32 {
    if year < 100 { 2000 + year } else { year }
}
