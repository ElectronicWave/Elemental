use reqwest::Url;

use super::util::origin_suffix;
use super::{Origin, OriginPolicy};
use crate::error::{Error, Result};

impl<O: Origin> OriginPolicy<O> {
    pub fn rewrite_origin_url(&self, raw_url: &str) -> Result<String> {
        let Some(rewritten) = self.try_rewrite_origin_url(raw_url)? else {
            return Err(Error::UnknownOrigin {
                raw_url: raw_url.to_owned(),
            });
        };

        Ok(rewritten)
    }

    pub fn try_rewrite_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| Error::invalid_url("parse url failed", raw_url, source))?;

        for origin in O::all() {
            let canonical = Url::parse(origin.canonical()).map_err(|source| {
                Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
            })?;
            if let Some(suffix) = origin_suffix(&parsed, &canonical) {
                return self.resolve(*origin, &suffix).map(Some);
            }

            if let Some(override_base) = self.overrides.get(origin)
                && let Some(suffix) = origin_suffix(&parsed, override_base)
            {
                return self.resolve(*origin, &suffix).map(Some);
            }
        }

        Ok(None)
    }
}
