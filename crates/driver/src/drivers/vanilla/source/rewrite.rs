use crate::error::{Error, Result};

use super::VanillaEndpoints;

pub fn rewrite_upstream_with_vanilla_fallback<RewriteFn>(
    vanilla_endpoints: &VanillaEndpoints,
    raw_url: &str,
    family_name: &str,
    rewrite_family: RewriteFn,
) -> Result<String>
where
    RewriteFn: FnOnce() -> Result<String>,
{
    if let Ok(rewritten) = vanilla_endpoints.rewrite_upstream(raw_url) {
        return Ok(rewritten);
    }

    rewrite_family().map_err(|source| Error::RewriteUpstream {
        family_name: family_name.to_owned(),
        raw_url: raw_url.to_owned(),
        source: Box::new(source),
    })
}
