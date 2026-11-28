use reqwest::Url;

use super::{
    OriginPolicy,
    error::{OriginError, Result},
    util::{origin_suffix, same_origin},
};

pub(super) enum OriginMatch {
    NotMatched,
    MatchedNoSuffix,
    Resolved(String),
}

impl OriginPolicy {
    pub fn rewrite_origin_url(&self, known_origins: &[&str], raw_url: &str) -> Result<String> {
        let Some(rewritten) = self.try_rewrite_origin_url(known_origins, raw_url)? else {
            return Err(OriginError::UnknownOrigin {
                raw_url: raw_url.to_owned(),
            });
        };

        Ok(rewritten)
    }

    pub fn try_rewrite_origin_url(
        &self,
        known_origins: &[&str],
        raw_url: &str,
    ) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| OriginError::invalid_url("parse url failed", raw_url, source))?;

        for &canonical in known_origins {
            if let OriginMatch::Resolved(rewritten) = self.match_origin(&parsed, canonical)? {
                return Ok(Some(rewritten));
            }
        }

        Ok(None)
    }

    pub(super) fn match_origin(&self, parsed: &Url, canonical_raw: &str) -> Result<OriginMatch> {
        let canonical = Url::parse(canonical_raw).map_err(|source| {
            OriginError::invalid_url("parse canonical origin failed", canonical_raw, source)
        })?;

        if let Some(suffix) = origin_suffix(parsed, &canonical) {
            return self
                .resolve(canonical_raw, &suffix)
                .map(OriginMatch::Resolved);
        }

        if let Some(override_base) = self.override_for(canonical_raw) {
            if let Some(suffix) = origin_suffix(parsed, override_base) {
                return self
                    .resolve(canonical_raw, &suffix)
                    .map(OriginMatch::Resolved);
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
