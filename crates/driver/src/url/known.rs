use reqwest::Url;

use super::rewrite::OriginMatch;
use super::{Origin, OriginPolicy};
use crate::error::{Error, Result};

impl<O: Origin> OriginPolicy<O> {
    pub fn rewrite_known_origin_url(&self, raw_url: &str) -> Result<Option<String>> {
        let parsed = Url::parse(raw_url)
            .map_err(|source| Error::invalid_url("parse url failed", raw_url, source))?;
        let mut matched_known_origin = false;

        for origin in O::all() {
            match self.match_origin(&parsed, *origin)? {
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

    pub fn rewrite_known_origin_url_or_keep(&self, raw_url: &str) -> Result<String> {
        if let Some(rewritten) = self.rewrite_known_origin_url(raw_url)? {
            return Ok(rewritten);
        }

        Ok(raw_url.to_owned())
    }
}
