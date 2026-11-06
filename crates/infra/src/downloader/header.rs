use std::fmt;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::{Error, Result};

pub(crate) struct HeaderNames<'a>(pub(crate) &'a HeaderMap);

impl fmt::Debug for HeaderNames<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.keys().map(HeaderName::as_str))
            .finish()
    }
}

pub(crate) fn insert_header(headers: &mut HeaderMap, name: &str, value: &str) -> Result<()> {
    let header_name =
        HeaderName::from_bytes(name.as_bytes()).map_err(|error| Error::InvalidHeaderName {
            name: name.to_owned(),
            reason: error.to_string(),
        })?;
    let header_value = HeaderValue::from_str(value).map_err(|error| Error::InvalidHeaderValue {
        name: name.to_owned(),
        reason: error.to_string(),
    })?;
    headers.append(header_name, header_value);
    Ok(())
}

pub(crate) fn merge_headers(base: &HeaderMap, overrides: &HeaderMap) -> HeaderMap {
    if overrides.is_empty() {
        return base.clone();
    }

    if base.is_empty() {
        return overrides.clone();
    }

    let mut merged = base.clone();
    for name in overrides.keys() {
        merged.remove(name);
    }
    for (name, value) in overrides {
        merged.append(name.clone(), value.clone());
    }
    merged
}
