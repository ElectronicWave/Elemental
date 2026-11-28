use elemental_core::url::{OriginPolicy, Result as OriginResult};
use elemental_driver::{
    error::Result as DriverResult, families::version_json::UpstreamUrlRewriter,
};

use super::endpoints::{KNOWN_LITELOADER_ORIGINS, LiteLoaderEndpoints, LiteLoaderOrigin};

const LEGACY_ORIGINS: &[(&str, LiteLoaderOrigin)] = &[
    (
        "http://dl.liteloader.com/versions",
        LiteLoaderOrigin::Downloads,
    ),
    (
        "http://repo.mumfrey.com/content/repositories/snapshots",
        LiteLoaderOrigin::Snapshots,
    ),
    (
        "http://repo.spongepowered.org/maven",
        LiteLoaderOrigin::SpongeMaven,
    ),
    (
        "http://repo.maven.apache.org/maven2",
        LiteLoaderOrigin::MavenCentral,
    ),
    (
        "http://repo1.maven.org/maven2",
        LiteLoaderOrigin::MavenCentral,
    ),
];

impl LiteLoaderEndpoints {
    pub fn rewrite_upstream(&self, raw_url: &str) -> crate::error::Result<String> {
        for (legacy_base, origin) in LEGACY_ORIGINS {
            if let Some(rewritten) =
                rewrite_legacy_origin(&self.origin_policy, raw_url, legacy_base, *origin)?
            {
                return Ok(rewritten);
            }
        }

        Ok(self
            .origin_policy
            .rewrite_known_origin_url_or_keep(KNOWN_LITELOADER_ORIGINS, raw_url)?)
    }
}

impl UpstreamUrlRewriter for LiteLoaderEndpoints {
    fn rewrite_upstream(&self, raw_url: &str) -> DriverResult<String> {
        for (legacy_base, origin) in LEGACY_ORIGINS {
            if let Some(rewritten) =
                rewrite_legacy_origin(&self.origin_policy, raw_url, legacy_base, *origin)?
            {
                return Ok(rewritten);
            }
        }

        Ok(self
            .origin_policy
            .rewrite_known_origin_url_or_keep(KNOWN_LITELOADER_ORIGINS, raw_url)?)
    }
}

fn rewrite_legacy_origin(
    policy: &OriginPolicy,
    raw_url: &str,
    legacy_base: &str,
    origin: LiteLoaderOrigin,
) -> OriginResult<Option<String>> {
    let trimmed_legacy_base = legacy_base.trim_end_matches('/');
    let Some(suffix) = raw_url.strip_prefix(trimmed_legacy_base) else {
        return Ok(None);
    };

    policy.resolve(origin, suffix).map(Some)
}
