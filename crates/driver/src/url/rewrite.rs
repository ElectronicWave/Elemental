use reqwest::Url;

use super::util::{origin_suffix, same_origin};
use super::{Origin, OriginPolicy};
use crate::error::{Error, Result};

pub(super) enum OriginMatch {
    NotMatched,
    MatchedNoSuffix,
    Resolved(String),
}

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
            if let OriginMatch::Resolved(rewritten) = self.match_origin(&parsed, *origin)? {
                return Ok(Some(rewritten));
            }
        }

        Ok(None)
    }

    pub(super) fn match_origin(&self, parsed: &Url, origin: O) -> Result<OriginMatch> {
        let canonical = Url::parse(origin.canonical()).map_err(|source| {
            Error::invalid_url("parse canonical origin failed", origin.canonical(), source)
        })?;

        if let Some(suffix) = origin_suffix(parsed, &canonical) {
            return self.resolve(origin, &suffix).map(OriginMatch::Resolved);
        }

        if let Some(override_base) = self.overrides.get(&origin) {
            if let Some(suffix) = origin_suffix(parsed, override_base) {
                return self.resolve(origin, &suffix).map(OriginMatch::Resolved);
            }

            if same_origin(parsed, override_base) {
                return Ok(OriginMatch::MatchedNoSuffix);
            }
        }

        if same_origin(parsed, &canonical) {
            return Ok(OriginMatch::MatchedNoSuffix);
        }

        Ok(OriginMatch::NotMatched)
    }
}
