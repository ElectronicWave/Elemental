use reqwest::Url;

use super::util::{origin_suffix, same_origin};
use super::{Origin, OriginPolicy};
use crate::error::{Error, Result};

impl<O: Origin> OriginPolicy<O> {
    pub fn rewrite_known_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| Error::invalid_url("parse url failed", raw_url, source))?;
        let mut matched_known_origin = false;

        for origin in O::all() {
            let canonical = Url::parse(origin.canonical()).map_err(|source| {
                Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
            })?;

            if same_origin(&parsed, &canonical) {
                matched_known_origin = true;
                if let Some(suffix) = origin_suffix(&parsed, &canonical) {
                    return self.resolve(*origin, &suffix).map(Some);
                }
            }

            if let Some(override_base) = self.overrides.get(origin)
                && same_origin(&parsed, override_base)
            {
                matched_known_origin = true;
                if let Some(suffix) = origin_suffix(&parsed, override_base) {
                    return self.resolve(*origin, &suffix).map(Some);
                }
            }
        }

        if matched_known_origin {
            return Err(Error::OriginBasePathMismatch {
                raw_url: raw_url.to_owned(),
            });
        }

        Ok(None)
    }

    pub fn rewrite_known_origin_url_or_keep(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}
