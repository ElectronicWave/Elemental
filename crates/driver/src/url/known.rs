use reqwest::Url;

use super::OriginPolicy;
use super::rewrite::OriginMatch;
use crate::error::{Error, Result};

impl OriginPolicy {
    pub fn rewrite_known_origin_url(
        &self,
        known_origins: &[&str],
        raw_url: &str,
    ) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| Error::invalid_url("parse url failed", raw_url, source))?;
        let mut matched_known_origin = false;

        for &canonical in known_origins {
            match self.match_origin(&parsed, canonical)? {
                OriginMatch::Resolved(rewritten) => return Ok(Some(rewritten)),
                OriginMatch::MatchedNoSuffix => matched_known_origin = true,
                OriginMatch::NotMatched => {}
            }
        }

        if matched_known_origin {
            return Err(Error::OriginBasePathMismatch {
                raw_url: raw_url.to_owned(),
            });
        }

        Ok(None)
    }

    pub fn rewrite_known_origin_url_or_keep(
        &self,
        known_origins: &[&str],
        raw_url: &str,
    ) -> Result<String> {
        if let Some(rewritten) = self.rewrite_known_origin_url(known_origins, raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}
